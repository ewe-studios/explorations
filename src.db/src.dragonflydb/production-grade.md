---
title: "Production-Grade DragonflyDB"
subtitle: "Deployment, monitoring, and operations guide"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.dragonflydb
related: exploration.md, rust-revision.md
---

# Production-Grade DragonflyDB

## Overview

This document covers production deployment patterns, monitoring, backup strategies, and operational considerations for DragonflyDB.

## Part 1: Reference Architecture

### Single Node Deployment

```
┌─────────────────────────────────────────────────────────────┐
│                    Single Dragonfly Instance                 │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                   Dragonfly                          │    │
│  │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐          │    │
│  │  │Shard│ │Shard│ │Shard│ │Shard│ │Shard│  ...    │    │
│  │  │  0  │ │  1  │ │  2  │ │  3  │ │  4  │          │    │
│  │  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘          │    │
│  │                                                     │    │
│  │  Memory: 8GB                                       │    │
│  │  CPU: 4 cores                                      │    │
│  │  Throughput: ~500K QPS                            │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
│  Applications ──────> Port 6379                            │
│                                                             │
│  Monitoring ────────> Port 9090 (/metrics)                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Use cases:
- Development/Testing
- Small applications (< 100K QPS)
- Cache layer with Redis fallback
```

### High Availability Deployment

```
┌─────────────────────────────────────────────────────────────┐
│                 Dragonfly HA Architecture                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────┐                                       │
│  │   Application   │                                       │
│  └────────┬────────┘                                       │
│           │                                                 │
│     ┌─────┴─────┐                                           │
│     │           │                                           │
│     ▼           ▼                                           │
│ ┌────────┐  ┌────────┐                                     │
│ │  App   │  │  App   │                                     │
│ │Region 1│  │Region 2│                                     │
│ └───┬────┘  └───┬────┘                                     │
│     │           │                                          │
│     └─────┬─────┘                                          │
│           │                                                 │
│     ┌─────┴─────┐                                           │
│     │           │                                           │
│     ▼           ▼                                           │
│ ┌─────────┐  ┌─────────┐                                   │
│ │ Master  │─>│ Replica │  (async replication)              │
│ │Primary  │  │Secondary│                                   │
│ │ AZ-a    │  │  AZ-b   │                                   │
│ └─────────┘  └─────────┘                                   │
│     ▲              ▲                                        │
│     │              │                                        │
│ ┌──────────────────────────┐                                │
│ │   Load Balancer / VIP    │                                │
│ │   (failover routing)     │                                │
│ └──────────────────────────┘                                │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Failover process:
1. Health check detects master failure
2. Load balancer redirects traffic to replica
3. Replica promoted to master (manual or automated)
4. New replica provisioned
```

### Kubernetes Deployment

```yaml
# StatefulSet for Dragonfly with persistence
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: dragonfly
spec:
  serviceName: dragonfly
  replicas: 3
  selector:
    matchLabels:
      app: dragonfly
  template:
    metadata:
      labels:
        app: dragonfly
    spec:
      containers:
      - name: dragonfly
        image: docker.dragonflydb.io/dragonflydb/dragonfly:v1.13.0
        ports:
        - containerPort: 6379
          name: redis
        - containerPort: 8080
          name: http
        resources:
          requests:
            memory: "8Gi"
            cpu: "4"
          limits:
            memory: "16Gi"
            cpu: "8"
        args:
        - --maxmemory=12gb
        - --cache_mode=true
        - --logtostderr
        - --port=6379
        volumeMounts:
        - name: data
          mountPath: /data
        livenessProbe:
          tcpSocket:
            port: 6379
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          exec:
            command:
            - redis-cli
            - ping
          initialDelaySeconds: 5
          periodSeconds: 5
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: gp3
      resources:
        requests:
          storage: 50Gi
```

## Part 2: Terraform Configuration

### AWS Lambda Deployment

```hcl
# DragonflyDB on Lambda (for edge caching)
resource "aws_lambda_function" "dragonfly_handler" {
  filename         = "dragonfly-lambda.zip"
  function_name    = "dragonfly-edge-cache"
  role             = aws_iam_role.lambda_role.arn
  handler          = "dragonfly_handler"
  runtime          = "provided.al2023"
  architecture     = "arm64"  # Graviton2 for better performance
  memory_size      = 3008     # Max for Lambda
  timeout          = 900      # 15 minutes max

  environment {
    variables = {
      MAXMEMORY     = "2500"  # MB
      CACHE_MODE    = "true"
      LOG_LEVEL     = "info"
    }
  }

  # EFS for data persistence
  file_system_config {
    arn             = aws_efs_access_point.dragonfly_efs.arn
    local_mount_path = "/data"
  }

  vpc_config {
    subnet_ids         = aws_subnet.private[*].id
    security_group_ids = [aws_security_group.dragonfly.id]
  }

  tracing_config {
    mode = "Active"
  }
}

# EFS for persistence
resource "aws_efs_file_system" "dragonfly_storage" {
  encrypted = true
  performance_mode = "generalPurpose"
  throughput_mode  = "elastic"
}

resource "aws_efs_access_point" "dragonfly_efs" {
  file_system_id = aws_efs_file_system.dragonfly_storage.id

  posix_user {
    gid = 1000
    uid = 1000
  }

  root_directory {
    path = "/dragonfly"
    creation_info {
      owner_gid   = 1000
      owner_uid   = 1000
      permissions = "0755"
    }
  }
}

# API Gateway for Redis protocol over HTTP
resource "aws_api_gateway_rest_api" "dragonfly_api" {
  name = "dragonfly-api"
}

# CloudWatch log group
resource "aws_cloudwatch_log_group" "dragonfly_logs" {
  name              = "/aws/lambda/dragonfly-edge-cache"
  retention_in_days = 7
}

# IAM role
resource "aws_iam_role" "lambda_role" {
  name = "dragonfly-lambda-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "lambda.amazonaws.com"
      }
    }]
  })
}

# CloudWatch ElastiCache for primary
resource "aws_elasticache_cluster" "primary" {
  cluster_id           = "dragonfly-primary"
  engine               = "redis"
  node_type            = "cache.r6g.xlarge"
  num_cache_nodes      = 1
  parameter_group_name = "default.redis6.x"
  port                 = 6379
}
```

### EC2 Auto Scaling Group

```hcl
# DragonflyDB on EC2 with Auto Scaling
resource "aws_launch_template" "dragonfly" {
  name_prefix   = "dragonfly-"
  image_id      = data.aws_ami.amazon_linux_2.id
  instance_type = "r6g.2xlarge"  # Memory optimized

  network_interfaces {
    associate_public_ip_address = false
    security_groups             = [aws_security_group.dragonfly.id]
    subnet_id                   = aws_subnet.private.id
  }

  block_device_mappings {
    device_name = "/dev/xvda"

    ebs {
      volume_size           = 100
      volume_type           = "gp3"
      encrypted             = true
      delete_on_termination = true
    }
  }

  user_data = base64encode(<<-EOF
              #!/bin/bash
              yum update -y
              yum install -y docker
              systemctl start docker
              docker run -d --name dragonfly \
                -p 6379:6379 -p 8080:8080 \
                --memory=16g --cpus=4 \
                docker.dragonflydb.io/dragonflydb/dragonfly \
                --maxmemory=14gb \
                --cache_mode=true
              EOF
  )

  tag_specifications {
    resource_type = "instance"
    tags = {
      Name = "dragonfly"
    }
  }
}

resource "aws_autoscaling_group" "dragonfly" {
  name                = "dragonfly-asg"
  vpc_zone_identifier = aws_subnet.private[*].id
  target_group_arns   = [aws_lb_target_group.dragonfly.arn]
  health_check_type   = "ELB"
  min_size            = 1
  max_size            = 3
  desired_capacity    = 2

  launch_template {
    id      = aws_launch_template.dragonfly.id
    version = "$Latest"
  }
}

# Network Load Balancer
resource "aws_lb" "dragonfly" {
  name               = "dragonfly-nlb"
  internal           = true
  load_balancer_type = "network"
  subnets            = aws_subnet.private[*].id
}

resource "aws_lb_target_group" "dragonfly" {
  name     = "dragonfly-tg"
  port     = 6379
  protocol = "TCP"
  vpc_id   = aws_vpc.main.id

  health_check {
    port                = "6379"
    protocol            = "TCP"
    interval            = 30
    healthy_threshold   = 2
    unhealthy_threshold = 3
  }
}
```

## Part 3: Monitoring with Prometheus

### Key Metrics to Monitor

```
Memory Metrics:
- used_memory: Current memory usage in bytes
- used_memory_rss: Resident set size (OS perspective)
- used_memory_peak: Peak memory usage
- mem_fragmentation_ratio: RSS / used_memory
- mem_fragmentation_bytes: RSS - used_memory
- evicted_keys: Keys evicted due to maxmemory

Performance Metrics:
- instantaneous_ops_per_sec: Operations per second
- total_connections_received: Total connections
- rejected_connections: Connection rejections
- blocked_clients: Clients waiting on blocking ops

Replication Metrics:
- master_repl_offset: Master replication offset
- slave_repl_offset: Replica replication offset
- replication_lag_bytes: Difference between master/replica
- connected_slaves: Number of connected replicas

Keyspace Metrics:
- db0:keys: Number of keys in database 0
- db0:expires: Keys with expiry set
- db0:avg_ttl: Average TTL in milliseconds

Dragonfly-specific:
- dragonfly_shard_wired_entries: Entries per shard
- dragonfly_tx_queue_len: Transaction queue length
- dragonfly_pipelined_commands: Pipeline commands processed
```

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'dragonfly'
    static_configs:
      - targets: ['dragonfly:9090']  # Dragonfly metrics endpoint
    metrics_path: /metrics
    scrape_interval: 5s

  - job_name: 'dragonfly-replica'
    static_configs:
      - targets: ['dragonfly-replica:9090']
    metrics_path: /metrics
    scrape_interval: 5s

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['alertmanager:9093']

rule_files:
  - 'dragonfly_alerts.yml'
```

### Alert Rules

```yaml
# dragonfly_alerts.yml
groups:
  - name: dragonfly
    rules:
      # Memory alerts
      - alert: DragonflyHighMemory
        expr: used_memory / maxmemory > 0.85
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Dragonfly memory usage above 85%"
          description: "Memory usage is {{ $value | humanizePercentage }}"

      - alert: DragonflyCriticalMemory
        expr: used_memory / maxmemory > 0.95
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Dragonfly memory usage above 95%"
          description: "Immediate action required"

      - alert: DragonflyHighFragmentation
        expr: mem_fragmentation_ratio > 1.5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Dragonfly memory fragmentation high"
          description: "Fragmentation ratio: {{ $value }}"

      # Replication alerts
      - alert: DragonflyReplicationLag
        expr: master_repl_offset - slave_repl_offset > 10000000
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Dragonfly replication lag detected"
          description: "Lag: {{ $value }} bytes"

      - alert: DragonflyReplicaDown
        expr: connected_slaves == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Dragonfly replica disconnected"

      # Performance alerts
      - alert: DragonflyHighLatency
        expr: histogram_quantile(0.99, rate(command_latency_bucket[5m])) > 0.01
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Dragonfly P99 latency above 10ms"

      - alert: DragonflyConnectionErrors
        expr: rate(rejected_connections[5m]) > 10
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Dragonfly connection rejections"

      - alert: DragonflyEvictions
        expr: rate(evicted_keys[5m]) > 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High key eviction rate"
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "DragonflyDB Overview",
    "panels": [
      {
        "title": "Memory Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "used_memory",
            "legendFormat": "Used Memory"
          },
          {
            "expr": "used_memory_rss",
            "legendFormat": "RSS Memory"
          },
          {
            "expr": "maxmemory",
            "legendFormat": "Max Memory"
          }
        ]
      },
      {
        "title": "Operations per Second",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(instantaneous_ops_per_sec[1m])",
            "legendFormat": "OPS"
          }
        ]
      },
      {
        "title": "Replication Lag",
        "type": "graph",
        "targets": [
          {
            "expr": "master_repl_offset - slave_repl_offset",
            "legendFormat": "Lag (bytes)"
          }
        ]
      },
      {
        "title": "Keyspace",
        "type": "stat",
        "targets": [
          {
            "expr": "db0:keys",
            "legendFormat": "Total Keys"
          },
          {
            "expr": "db0:expires",
            "legendFormat": "Keys with TTL"
          }
        ]
      }
    ]
  }
}
```

## Part 4: Backup Strategy

### RDB Backup to S3

```bash
#!/bin/bash
# backup_dragonfly.sh

set -e

BACKUP_DIR="/tmp/dragonfly-backup"
S3_BUCKET="dragonfly-backups"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

# Create backup directory
mkdir -p $BACKUP_DIR

# Trigger BGSAVE
redis-cli BGSAVE

# Wait for BGSAVE to complete
while [ "$(redis-cli LASTSAVE)" == "$(cat /tmp/lastsave)" ]; do
    sleep 1
done
redis-cli LASTSAVE > /tmp/lastsave

# Copy RDB file
cp /data/dump.rdb $BACKUP_DIR/dump-$TIMESTAMP.rdb

# Compress
gzip $BACKUP_DIR/dump-$TIMESTAMP.rdb

# Upload to S3
aws s3 cp $BACKUP_DIR/dump-$TIMESTAMP.rdb.gz \
    s3://$S3_BUCKET/$(hostname)/dump-$TIMESTAMP.rdb.gz

# Upload to S3 with lifecycle policy
aws s3 cp $BACKUP_DIR/dump-$TIMESTAMP.rdb.gz \
    s3://$S3_BUCKET/$(hostname)/daily/dump-$TIMESTAMP.rdb.gz

# Keep only last 7 daily backups
aws s3 ls s3://$S3_BUCKET/$(hostname)/daily/ | \
    sort -r | awk 'NR>7 {print $4}' | \
    xargs -I {} aws s3 rm s3://$S3_BUCKET/$(hostname)/daily/{}

# Cleanup local
rm -rf $BACKUP_DIR
```

### Scheduled Backups with Cron

```hcl
# Terraform EventBridge rule for scheduled backups
resource "aws_cloudwatch_event_rule" "dragonfly_backup" {
  name                = "dragonfly-daily-backup"
  description         = "Daily Dragonfly backup"
  schedule_expression = "cron(0 2 * * ? *)"  # Daily at 2 AM UTC
}

resource "aws_cloudwatch_event_target" "dragonfly_backup" {
  rule      = aws_cloudwatch_event_rule.dragonfly_backup.name
  target_id = "dragonflyBackupLambda"
  arn       = aws_lambda_function.backup.arn
}

resource "aws_lambda_function" "backup" {
  filename         = "backup.zip"
  function_name    = "dragonfly-backup"
  role             = aws_iam_role.backup_role.arn
  handler          = "backup.handler"
  runtime          = "python3.9"
  timeout          = 300

  environment {
    variables = {
      DRAGONFLY_HOST = aws_lb.dragonfly.dns_name
      S3_BUCKET      = aws_s3_bucket.backups.bucket
    }
  }
}
```

## Part 5: Performance Tuning

### Kernel Parameters

```bash
# /etc/sysctl.conf

# Increase max connections
net.core.somaxconn = 65535

# Increase TCP backlog
net.ipv4.tcp_max_syn_backlog = 65535

# Reduce TCP timeout
net.ipv4.tcp_fin_timeout = 15

# Enable TCP keepalive
net.ipv4.tcp_keepalive_time = 300
net.ipv4.tcp_keepalive_intvl = 60
net.ipv4.tcp_keepalive_probes = 3

# Increase file descriptor limit
fs.file-max = 2097152

# Overcommit memory (required for Redis/Dragonfly)
vm.overcommit_memory = 1

# Disable transparent hugepages
vm.transparent_hugepage = never

# Apply settings
sysctl -p
```

### Dragonfly Configuration

```bash
# dragonfly.conf

# Network
port 6379
bind 0.0.0.0
timeout 300
tcp-keepalive 60

# Memory
maxmemory 14gb
maxmemory-policy volatile-lru  # Evict keys with TTL first

# Persistence
dir /data
dbfilename dump.rdb
save 900 1      # Save after 900 sec if 1 key changed
save 300 10     # Save after 300 sec if 10 keys changed
save 60 10000   # Save after 60 sec if 10000 keys changed

# Replication
replica-serve-stale-data yes
replica-read-only yes

# Performance
hz 100           # Key expiry frequency
activedefrag no  # Disable active defrag (not needed for Dragonfly)

# Logging
loglevel notice
logfile /var/log/dragonfly.log

# Security
requirepass your_strong_password_here
```

### Performance Benchmarks

```bash
# Benchmark with memtier_benchmark

# Test 1: SET throughput
memtier_benchmark \
  --server=dragonfly-host \
  --port=6379 \
  --protocol=redis \
  --ratio=1:0 \
  --test-time=60 \
  --threads=4 \
  --clients=100 \
  --data-size=256 \
  --key-prefix=set: \
  --hide-histogram

# Test 2: GET throughput
memtier_benchmark \
  --server=dragonfly-host \
  --port=6379 \
  --protocol=redis \
  --ratio=0:1 \
  --test-time=60 \
  --threads=4 \
  --clients=100 \
  --data-size=256 \
  --key-prefix=set: \
  --hide-histogram

# Test 3: Mixed workload
memtier_benchmark \
  --server=dragonfly-host \
  --port=6379 \
  --protocol=redis \
  --ratio=1:3 \
  --test-time=120 \
  --threads=8 \
  --clients=200 \
  --data-size=512 \
  --key-prefix=mixed: \
  --expiry-range=3600-7200

# Expected results (r6g.2xlarge, 4 cores):
# SET: ~400K QPS
# GET: ~600K QPS
# Mixed: ~500K QPS
```

---

*This document is part of the DragonflyDB exploration series. See [exploration.md](./exploration.md) for the complete index.*
