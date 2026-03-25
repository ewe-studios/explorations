# Production-Grade ZeroFS: Deployment and Operations Guide

**Source:** Analysis of ZeroFS documentation and related projects

---

## Table of Contents

1. [Production Architecture](#production-architecture)
2. [Deployment Options](#deployment-options)
3. [Configuration Guide](#configuration-guide)
4. [Monitoring and Observability](#monitoring-and-observability)
5. [Backup and Recovery](#backup-and-recovery)
6. [Performance Tuning](#performance-tuning)
7. [Troubleshooting](#troubleshooting)
8. [Security Considerations](#security-considerations)

---

## Production Architecture

### Reference Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Production ZeroFS Architecture                        │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   NFS Clients   │     │   9P Clients    │     │   NBD Clients   │
│   (10-100s)     │     │   (10-100s)     │     │   (ZFS, DBs)    │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         │ NFS over TCP          │ 9P over TCP           │ NBD over TCP
         │ port 2049             │ port 5564             │ port 10809
         ▼                       ▼                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      Load Balancer (Optional)                            │
│                    HAProxy / NGINX / AWS ALB                            │
└─────────────────────────────────────────────────────────────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      ZeroFS Server Cluster                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐         │
│  │   ZeroFS-1      │  │   ZeroFS-2      │  │   ZeroFS-3      │         │
│  │   (Read-Write)  │  │   (Read-Only)   │  │   (Read-Only)   │         │
│  │                 │  │                 │  │                 │         │
│  │  - NFS Server   │  │  - NFS Server   │  │  - NFS Server   │         │
│  │  - 9P Server    │  │  - 9P Server    │  │  - 9P Server    │         │
│  │  - NBD Server   │  │  - NBD Server   │  │  - NBD Server   │         │
│  │  - Compactor    │  │                 │  │                 │         │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘         │
│           │                    │                    │                   │
└───────────┼────────────────────┼────────────────────┼───────────────────┘
            │                    │                    │
            └────────────────────┼────────────────────┘
                                 │
            ┌────────────────────┼────────────────────┐
            ▼                    ▼                    ▼
    ┌───────────────┐    ┌───────────────┐    ┌───────────────┐
    │   AWS S3      │    │  Azure Blob   │    │   MinIO       │
    │   us-east-1   │    │  eu-west-1    │    │  (on-prem)    │
    └───────────────┘    └───────────────┘    └───────────────┘
```

### Component Sizing

| Component | Small | Medium | Large |
|-----------|-------|--------|-------|
| **CPU** | 2 cores | 4 cores | 8+ cores |
| **Memory** | 4 GB | 8 GB | 16+ GB |
| **Cache Disk** | 50 GB SSD | 200 GB SSD | 1 TB NVMe |
| **Network** | 1 Gbps | 10 Gbps | 25+ Gbps |
| **Max Clients** | 10-20 | 50-100 | 200+ |

---

## Deployment Options

### Docker Deployment

```yaml
# docker-compose.yml
version: '3.8'

services:
  zerofs:
    image: ghcr.io/barre/zerofs:latest
    container_name: zerofs
    restart: unless-stopped

    ports:
      - "2049:2049/tcp"  # NFS
      - "5564:5564/tcp"  # 9P
      - "10809:10809/tcp"  # NBD

    volumes:
      - ./zerofs.toml:/etc/zerofs.toml:ro
      - ./cache:/var/cache/zerofs
      - /data:/data

    environment:
      - ZEROFS_PASSWORD=${ZEROFS_PASSWORD}
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}

    ulimits:
      nofile:
        soft: 65536
        hard: 65536

    command: ["run", "-c", "/etc/zerofs.toml"]
```

### Kubernetes Deployment

```yaml
# zerofs-statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: zerofs
spec:
  serviceName: zerofs
  replicas: 3
  selector:
    matchLabels:
      app: zerofs
  template:
    metadata:
      labels:
        app: zerofs
    spec:
      containers:
      - name: zerofs
        image: ghcr.io/barre/zerofs:latest
        ports:
        - name: nfs
          containerPort: 2049
        - name: ninep
          containerPort: 5564
        - name: nbd
          containerPort: 10809
        volumeMounts:
        - name: config
          mountPath: /etc/zerofs.toml
          subPath: zerofs.toml
        - name: cache
          mountPath: /var/cache/zerofs
        env:
        - name: ZEROFS_PASSWORD
          valueFrom:
            secretKeyRef:
              name: zerofs-secrets
              key: password
        resources:
          requests:
            cpu: "2"
            memory: "4Gi"
          limits:
            cpu: "4"
            memory: "8Gi"
        livenessProbe:
          exec:
            command: ["zerofs", "health"]
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          exec:
            command: ["zerofs", "health"]
          initialDelaySeconds: 5
          periodSeconds: 5
  volumeClaimTemplates:
  - metadata:
      name: cache
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: ssd
      resources:
        requests:
          storage: 200Gi
---
apiVersion: v1
kind: Service
metadata:
  name: zerofs-nfs
spec:
  selector:
    app: zerofs
  ports:
  - port: 2049
    targetPort: 2049
    name: nfs
  type: LoadBalancer
```

### Systemd Service

```ini
# /etc/systemd/system/zerofs.service
[Unit]
Description=ZeroFS Distributed File System
After=network.target

[Service]
Type=notify
User=zerofs
Group=zerofs

EnvironmentFile=/etc/zerofs.env
ExecStart=/usr/local/bin/zerofs run -c /etc/zerofs.toml
ExecReload=/bin/kill -HUP $MAINPID

Restart=on-failure
RestartSec=5

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/cache/zerofs /data

# Resource limits
LimitNOFILE=65536
LimitNPROC=65536

[Install]
WantedBy=multi-user.target
```

---

## Configuration Guide

### Full Configuration Example

```toml
# zerofs.toml

# =====================
# Cache Configuration
# =====================
[cache]
dir = "/var/cache/zerofs"
disk_size_gb = 100.0
memory_size_gb = 4.0  # 0 to disable memory cache

# =====================
# Storage Configuration
# =====================
[storage]
url = "s3://my-bucket/zerofs-data"
encryption_password = "${ZEROFS_PASSWORD}"

# Optional: limit filesystem size
# max_size_gb = 1000.0

# Compression: "lz4" (default) or "zstd-{1-22}"
compression = "lz4"

# =====================
# LSM Tree Configuration
# =====================
[lsm]
max_concurrent_compactions = 4
memtable_size_mb = 256
l0_sst_size_mb = 64

# =====================
# Server Configuration
# =====================
[servers.nfs]
addresses = ["0.0.0.0:2049"]
# For IPv6: addresses = ["[::]:2049"]

[servers.ninep]
addresses = ["0.0.0.0:5564"]
unix_socket = "/tmp/zerofs.9p.sock"  # Optional

[servers.nbd]
addresses = ["0.0.0.0:10809"]
unix_socket = "/tmp/zerofs.nbd.sock"  # Optional

[servers.rpc]
addresses = ["127.0.0.1:7000"]
unix_socket = "/tmp/zerofs.rpc.sock"

# =====================
# AWS Configuration
# =====================
[aws]
access_key_id = "${AWS_ACCESS_KEY_ID}"
secret_access_key = "${AWS_SECRET_ACCESS_KEY}"
# endpoint = "https://s3.us-east-1.amazonaws.com"
# default_region = "us-east-1"
# allow_http = false  # For non-HTTPS (MinIO)

# =====================
# Azure Configuration
# =====================
# [azure]
# storage_account_name = "${AZURE_STORAGE_ACCOUNT_NAME}"
# storage_account_key = "${AZURE_STORAGE_ACCOUNT_KEY}"

# =====================
# GCP Configuration
# =====================
# [gcp]
# service_account = "/path/to/service-account.json"
```

### Environment File

```bash
# /etc/zerofs.env
ZEROFS_PASSWORD="your-secure-password-here"
AWS_ACCESS_KEY_ID="AKIAIOSFODNN7EXAMPLE"
AWS_SECRET_ACCESS_KEY="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
```

---

## Monitoring and Observability

### Metrics Endpoints

ZeroFS exposes Prometheus-compatible metrics:

```rust
// Available metrics
zerofs_operations_total{operation="read|write|lookup|..."}
zerofs_operation_duration_seconds{operation="...", quantile="0.5|0.9|0.99"}
zerofs_cache_hits_total{cache="memory|disk"}
zerofs_cache_size_bytes{cache="memory|disk"}
zerofs_s3_requests_total{operation="get|put|delete"}
zerofs_s3_request_duration_seconds{operation="..."}
zerofs_compaction_jobs_total
zerofs_compaction_duration_seconds
zerofs_encryption_operations_total
zerofs_open_files
zerofs_filesystem_size_bytes
zerofs_filesystem_used_bytes
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "ZeroFS Overview",
    "panels": [
      {
        "title": "Operations per Second",
        "targets": [
          {
            "expr": "rate(zerofs_operations_total[1m])",
            "legendFormat": "{{operation}}"
          }
        ]
      },
      {
        "title": "Operation Latency (p99)",
        "targets": [
          {
            "expr": "histogram_quantile(0.99, rate(zerofs_operation_duration_seconds_bucket[5m]))",
            "legendFormat": "{{operation}}"
          }
        ]
      },
      {
        "title": "Cache Hit Rate",
        "targets": [
          {
            "expr": "rate(zerofs_cache_hits_total[5m]) / (rate(zerofs_cache_hits_total[5m]) + rate(zerofs_cache_misses_total[5m]))",
            "legendFormat": "{{cache}}"
          }
        ]
      },
      {
        "title": "Filesystem Usage",
        "targets": [
          {
            "expr": "zerofs_filesystem_used_bytes / zerofs_filesystem_size_bytes * 100",
            "legendFormat": "Usage %"
          }
        ]
      }
    ]
  }
}
```

### Logging Configuration

```toml
# Enable debug logging
RUST_LOG=zerofs=debug,slatedb=info

# Structured logging (JSON format)
RUST_LOG_FORMAT=json

# Log file
RUST_LOG_FILE=/var/log/zerofs/zerofs.log
```

---

## Backup and Recovery

### Checkpoint-Based Backup

```bash
# Create a checkpoint (snapshot)
zerofs checkpoint create -c zerofs.toml backup-2024-01-15

# List checkpoints
zerofs checkpoint list -c zerofs.toml

# Get checkpoint info
zerofs checkpoint info -c zerofs.toml backup-2024-01-15

# Restore from checkpoint (read-only)
zerofs run -c zerofs.toml --checkpoint backup-2024-01-15
```

### S3 Bucket Lifecycle

```bash
# Enable versioning on S3 bucket
aws s3api put-bucket-versioning \
  --bucket my-bucket \
  --versioning-configuration Status=Enabled

# Enable lifecycle rules for cost optimization
aws s3api put-bucket-lifecycle-configuration \
  --bucket my-bucket \
  --lifecycle-configuration '{
    "Rules": [
      {
        "ID": "TransitionToIA",
        "Status": "Enabled",
        "Filter": {"Prefix": "zerofs-data/"},
        "Transitions": [
          {
            "Days": 30,
            "StorageClass": "STANDARD_IA"
          },
          {
            "Days": 90,
            "StorageClass": "GLACIER"
          }
        ]
      }
    ]
  }'
```

### Disaster Recovery

```bash
# Scenario: Complete region failure

# 1. Deploy ZeroFS in new region
# 2. Point to existing S3 bucket (cross-region)
# 3. Start in read-only mode first
zerofs run -c zerofs.toml --read-only

# 4. Verify data integrity
zerofs fsck -c zerofs.toml

# 5. Promote to read-write if primary is truly dead
zerofs run -c zerofs.toml
```

---

## Performance Tuning

### Cache Tuning

```toml
# Memory cache sizing
# Rule of thumb: 25% of available RAM for hot data
[cache]
memory_size_gb = 8.0  # For 32 GB RAM system

# Disk cache sizing
# Rule of thumb: 10-20% of total dataset size
disk_size_gb = 500.0

# Cache eviction
# Larger values = longer retention but more memory
```

### Network Tuning

```bash
# Increase TCP buffer sizes
sysctl -w net.core.rmem_max=16777216
sysctl -w net.core.wmem_max=16777216
sysctl -w net.ipv4.tcp_rmem="4096 87380 16777216"
sysctl -w net.ipv4.tcp_wmem="4096 87380 16777216"

# Increase connection backlog
sysctl -w net.core.somaxconn=65535

# Enable TCP keepalive
sysctl -w net.ipv4.tcp_keepalive_time=60
sysctl -w net.ipv4.tcp_keepalive_intvl=10
sysctl -w net.ipv4.tcp_keepalive_probes=6
```

### Compaction Tuning

```toml
# For write-heavy workloads
[lsm]
max_concurrent_compactions = 8  # Increase parallelism
memtable_size_mb = 512  # Larger memtables = fewer flushes
l0_sst_size_mb = 128  # Larger SSTs = better compression

# For read-heavy workloads
[lsm]
max_concurrent_compactions = 2  # Less compaction interference
memtable_size_mb = 128
l0_sst_size_mb = 32  # Smaller SSTs = faster reads
```

---

## Troubleshooting

### Common Issues

**Issue: High Latency**
```bash
# Check cache hit rate
curl http://localhost:9090/metrics | grep cache_hits

# Check S3 latency
aws s3 cp /dev/null s3://bucket/test --dryrun

# Check compaction queue
zerofs metrics -c zerofs.toml | grep compaction
```

**Issue: Out of Space**
```bash
# Check filesystem usage
zerofs df -c zerofs.toml

# Check cache usage
du -sh /var/cache/zerofs

# Clear cache (safe, will repopulate)
rm -rf /var/cache/zerofs/*
```

**Issue: Mount Failures**
```bash
# Check if port is in use
netstat -tlnp | grep 2049

# Check FUSE module
lsmod | grep fuse

# Check permissions
ls -la /dev/fuse
```

### Debug Commands

```bash
# Enable debug logging
RUST_LOG=zerofs=debug zerofs run -c zerofs.toml

# Run health check
zerofs health -c zerofs.toml

# Check database integrity
zerofs fsck -c zerofs.toml

# Profile performance
cargo flamegraph --bin zerofs -- run -c zerofs.toml

# Trace system calls
strace -f -p $(pgrep zerofs)
```

---

## Security Considerations

### Network Security

```bash
# Firewall rules (iptables)
# Allow NFS from trusted networks
iptables -A INPUT -p tcp -s 10.0.0.0/8 --dport 2049 -j ACCEPT
iptables -A INPUT -p tcp -s 192.168.0.0/16 --dport 2049 -j ACCEPT

# Allow 9P from trusted networks
iptables -A INPUT -p tcp -s 10.0.0.0/8 --dport 5564 -j ACCEPT

# Block all other access
iptables -A INPUT -p tcp --dport 2049 -j DROP
iptables -A INPUT -p tcp --dport 5564 -j DROP
iptables -A INPUT -p tcp --dport 10809 -j DROP
```

### Encryption at Rest

```toml
# Always enabled in ZeroFS
[storage]
encryption_password = "${ZEROFS_PASSWORD}"

# For additional security, layer with filesystem encryption
# Mount gocryptfs on top of ZeroFS
gocryptfs /mnt/zerofs /mnt/encrypted -plaintextnames
```

### Access Control

```bash
# NFS export options
# /etc/exports
/mnt/zerofs 10.0.0.0/8(rw,sync,no_subtree_check,no_root_squash)
/mnt/zerofs 192.168.0.0/16(ro,sync,no_subtree_check)

# 9P permissions
# Set in zerofs.toml or via POSIX permissions on files
chown -R users:users /mnt/zerofs
chmod -R 750 /mnt/zerofs
```

---

## Summary

### Production Checklist

- [ ] Configure appropriate cache sizes
- [ ] Set up monitoring and alerting
- [ ] Configure backup/checkpoint strategy
- [ ] Tune network parameters
- [ ] Set up log aggregation
- [ ] Configure firewall rules
- [ ] Test failover procedures
- [ ] Document runbooks
- [ ] Set up on-call rotation

### Key Takeaways

1. **Deployment**: Docker, Kubernetes, or systemd
2. **Configuration**: Tune cache, compaction, network
3. **Monitoring**: Prometheus metrics, Grafana dashboards
4. **Backup**: Checkpoints + S3 versioning
5. **Performance**: Cache sizing, parallelism, network tuning
6. **Security**: Network isolation, encryption, access control

### Further Reading

- [ZeroFS Documentation](https://www.zerofs.net)
- [SlateDB Operations Guide](https://github.com/slatedb/slatedb)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
