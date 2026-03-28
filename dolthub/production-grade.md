---
title: "Production-Grade Dolt: Operating at Scale"
subtitle: "Building, deploying, and operating Dolt in production environments"
based_on: "Dolt storage architecture with production enhancements"
level: "Advanced - Production engineering considerations"
---

# Production-Grade Dolt: Operations and Scaling Guide

## Table of Contents

1. [Production Architecture Overview](#1-production-architecture-overview)
2. [High Availability Design](#2-high-availability-design)
3. [Scaling Strategies](#3-scaling-strategies)
4. [Performance Tuning](#4-performance-tuning)
5. [Backup and Recovery](#5-backup-and-recovery)
6. [Monitoring and Observability](#6-monitoring-and-observability)
7. [Security Hardening](#7-security-hardening)
8. [Multi-tenant Deployments](#8-multi-tenant-deployments)

---

## 1. Production Architecture Overview

### 1.1 Deployment Patterns

**Single Node (Development/Testing):**
```
┌─────────────────────────────────────┐
│         Single Dolt Server          │
│  ┌─────────────────────────────┐   │
│  │  Dolt SQL Server            │   │
│  │  (MySQL-compatible)         │   │
│  └───────────┬─────────────────┘   │
│              │                     │
│  ┌───────────▼─────────────────┐   │
│  │  .dolt/                     │   │
│  │  ├── noms/ (storage)        │   │
│  │  ├── refs/ (branches)       │   │
│  │  └── state/ (working sets)  │   │
│  └─────────────────────────────┘   │
└─────────────────────────────────────┘
```

**High Availability Cluster:**
```
┌─────────────────────────────────────────────────────────────────┐
│                      LOAD BALANCER                               │
│              (HAProxy / nginx / cloud LB)                        │
└───────────────────────────┬─────────────────────────────────────┘
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
  ┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐
  │  Dolt-1     │   │  Dolt-2     │   │  Dolt-3     │
  │  (Primary)  │   │  (Replica)  │   │  (Replica)  │
  └──────┬──────┘   └──────┬──────┘   └──────┬──────┘
         │                 │                 │
         └─────────────────┴─────────────────┘
                           │
         ┌─────────────────▼─────────────────┐
         │       Shared Storage Layer         │
         │  ┌─────────────┐ ┌─────────────┐  │
         │  │   AWS S3    │ │  DynamoDB   │  │
         │  │  (Chunks)   │ │  (Manifest) │  │
         │  └─────────────┘ └─────────────┘  │
         └───────────────────────────────────┘
```

**DoltHub-Style Multi-tenant:**
```
┌─────────────────────────────────────────────────────────────────┐
│                         API GATEWAY                              │
└───────────────────────────┬─────────────────────────────────────┘
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
  ┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐
  │   Tenant-1  │   │   Tenant-2  │   │   Tenant-N  │
  │   Cluster   │   │   Cluster   │   │   Cluster   │
  └─────────────┘   └─────────────┘   └─────────────┘
         │                 │                  │
         └─────────────────┴──────────────────┘
                           │
         ┌─────────────────▼─────────────────┐
         │     Object Storage (S3/GCS)        │
         │     Per-tenant bucket isolation    │
         └───────────────────────────────────┘
```

### 1.2 Component Sizing

| Component | Small | Medium | Large | XLarge |
|-----------|-------|--------|-------|--------|
| **CPU** | 2 cores | 4 cores | 8 cores | 16+ cores |
| **RAM** | 4 GB | 8 GB | 16 GB | 64+ GB |
| **Storage** | 50 GB SSD | 200 GB SSD | 500 GB NVMe | 2+ TB NVMe |
| **Use Case** | Dev/Test | Small team | Production | Enterprise |

---

## 2. High Availability Design

### 2.1 Replication Strategies

**Async Replica (Recommended):**
```sql
-- On replica server
dolt sql -q "CALL dolt_clone('s3://bucket/dolt-repo', 'replica-db');"

-- Configure replication lag monitoring
SELECT * FROM dolt_status;
```

**Push-Pull Replication:**
```bash
#!/bin/bash
# replication-sync.sh
while true; do
    dolt pull origin main
    dolt push replica main:main
    sleep 60
done
```

### 2.2 Failover Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    HEALTH CHECKER                            │
│              (checks every 5 seconds)                        │
└─────────────────────┬───────────────────────────────────────┘
                      │
         ┌────────────┴────────────┐
         │                         │
  ┌──────▼──────┐           ┌──────▼──────┐
  │  Primary    │           │   Standby   │
  │  ✅ Healthy │           │  ⏸ Waiting  │
  └─────────────┘           └──────┬──────┘
                                   │
                          ┌────────▼────────┐
                          │ Virtual IP / DNS │
                          │  dolt.example.com│
                          └─────────────────┘
```

**Failover Script:**
```bash
#!/bin/bash
# failover.sh
PRIMARY="dolt-primary"
STANDBY="dolt-standby"

if ! health_check $PRIMARY; then
    echo "Primary failed, initiating failover..."

    # Promote standby
    ssh $STANDBY "dolt sql -q 'CALL dolt_backup promote'"

    # Update DNS
    update_dns "dolt.example.com" $STANDBY

    # Alert on-call
    send_alert "Dolt failover completed to $STANDBY"
fi
```

### 2.3 Backup Strategies

**Continuous Backup:**
```bash
# Continuous backup to S3
dolt backup add s3-backup s3://bucket/backups/$(date +%Y%m%d)
dolt backup sync s3-backup
```

**Point-in-Time Recovery:**
```sql
-- List available snapshots
SELECT * FROM dolt_log;

-- Create branch from specific commit
CALL dolt_checkout('-b', 'recovery-branch', 'abc123def');

-- Export recovered data
SELECT * FROM recovered_table INTO OUTFILE '/tmp/recovery.csv';
```

---

## 3. Scaling Strategies

### 3.1 Horizontal Scaling

**Read Replicas:**
```
                     ┌─────────────┐
                     │   Reader    │
                     │  Endpoint   │
                     └──────┬──────┘
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
  ┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐
  │  Read-1     │   │  Read-2     │   │  Read-3     │
  │  (RO)       │   │  (RO)       │   │  (RO)       │
  └─────────────┘   └─────────────┘   └─────────────┘
```

**Sharding by Database:**
```yaml
# dolt-shard-config.yaml
shards:
  - name: shard-1
    databases: [users, auth, sessions]
    endpoint: shard-1.dolt.internal
  - name: shard-2
    databases: [orders, inventory, shipping]
    endpoint: shard-2.dolt.internal
  - name: shard-3
    databases: [analytics, events, logs]
    endpoint: shard-3.dolt.internal
```

### 3.2 Vertical Scaling

**Memory Configuration:**
```bash
# Tune chunk cache size
dolt config --global --add metrics.host_stats true
dolt config --global --add pprof.port 6060
```

**Storage Optimization:**
```sql
-- Optimize table storage
CALL dolt_optimize_tables('large_table');

-- Garbage collect old versions
CALL dolt_gc();
```

### 3.3 Connection Pooling

**Using ProxySQL:**
```sql
-- Configure connection pool
INSERT INTO mysql_servers (hostgroup_id, hostname, port)
VALUES (1, 'dolt-primary', 3306);

INSERT INTO mysql_users (username, password, default_hostgroup)
VALUES ('dolt_user', 'password', 1);

LOAD MYSQL SERVERS TO RUNTIME;
SAVE MYSQL SERVERS TO DISK;
```

---

## 4. Performance Tuning

### 4.1 Configuration Tuning

**Recommended Production Settings:**
```yaml
# ~/.dolt/config.yaml
metrics:
  host_stats: true
  prometheus:
    port: 9090

sql_server:
  max_connections: 100
  read_timeout_millis: 30000
  write_timeout_millis: 30000

store:
  chunk_cache_mb: 512
  batch_size: 1000
```

### 4.2 Query Optimization

**Analyze Slow Queries:**
```sql
-- Enable slow query log
SET GLOBAL log_slow_queries = ON;
SET GLOBAL long_query_time = 1.0;

-- Review slow queries
SELECT * FROM mysql.slow_log;

-- Use EXPLAIN
EXPLAIN SELECT * FROM users WHERE id = 1;
```

**Index Optimization:**
```sql
-- Create indexes for common queries
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_orders_date ON orders(order_date);

-- Analyze index usage
SELECT * FROM dolt_indexes;
```

### 4.3 Bulk Operations

**Batch Inserts:**
```sql
-- Efficient: Single transaction with batch
START TRANSACTION;
INSERT INTO users (name, email) VALUES
    ('Alice', 'alice@example.com'),
    ('Bob', 'bob@example.com'),
    ...  -- 1000 rows
    ('Zoe', 'zoe@example.com');
COMMIT;

-- Inefficient: Individual inserts
INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');
INSERT INTO users (name, email) VALUES ('Bob', 'bob@example.com');
-- ... repeated 1000 times
```

**Parallel Imports:**
```bash
#!/bin/bash
# parallel-import.sh
FILES=$(ls data/*.csv)
for file in $FILES; do
    (
        table=$(basename $file .csv)
        dolt sql -q "LOAD DATA INFILE '$file' INTO TABLE $table"
    ) &
done
wait
```

---

## 5. Backup and Recovery

### 5.1 Backup Strategies

**Full Backup with Verification:**
```bash
#!/bin/bash
# daily-backup.sh
BACKUP_DIR="/backups/dolt/$(date +%Y%m%d)"
mkdir -p $BACKUP_DIR

# Export all databases
for db in $(dolt sql -q "SHOW DATABASES" -r csv); do
    dolt --use-db=$db backup add daily $BACKUP_DIR/$db
    dolt --use-db=$db backup sync daily
done

# Verify backup integrity
for db in $(dolt sql -q "SHOW DATABASES" -r csv); do
    dolt --use-db=$db backup verify daily
done
```

**Incremental Backup:**
```bash
# Backup only changes since last backup
dolt backup add incremental s3://bucket/inc/
dolt backup sync incremental --since LAST_BACKUP_HASH
```

### 5.2 Disaster Recovery Plan

**RTO/RPO Targets:**

| Scenario | RTO | RPO | Strategy |
|----------|-----|-----|----------|
| Single node failure | 5 min | 1 min | Auto-failover |
| Region failure | 30 min | 5 min | Cross-region replica |
| Data corruption | 1 hour | 1 day | Point-in-time recovery |
| Complete loss | 4 hours | 1 week | Full restore from backup |

**Recovery Runbook:**
```bash
# 1. Assess damage
dolt sql -q "SELECT * FROM dolt_status;"

# 2. Identify last good commit
dolt log --oneline

# 3. Create recovery branch
dolt checkout -b recovery GOOD_COMMIT_HASH

# 4. Verify data integrity
dolt diff recovery main

# 5. Promote recovery branch
dolt checkout main
dolt merge recovery

# 6. Notify stakeholders
notify "Database recovered from $GOOD_COMMIT_HASH"
```

---

## 6. Monitoring and Observability

### 6.1 Metrics Collection

**Prometheus Configuration:**
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'dolt'
    static_configs:
      - targets: ['dolt-1:9090', 'dolt-2:9090', 'dolt-3:9090']
    metrics_path: /metrics
```

**Key Metrics:**

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `dolt_connections_active` | Active connections | > 80% max |
| `dolt_query_duration_seconds` | Query latency | p99 > 1s |
| `dolt_storage_size_bytes` | Storage usage | > 85% capacity |
| `dolt_replication_lag_seconds` | Replica lag | > 30s |
| `dolt_gc_duration_seconds` | GC duration | > 5 min |

### 6.2 Logging Configuration

**Structured Logging:**
```yaml
# logging.yaml
logging:
  level: INFO
  format: json
  outputs:
    - stdout
    - file:/var/log/dolt/dolt.log
  rotation:
    max_size: 100MB
    max_backups: 5
    max_age: 30
```

**Log Aggregation:**
```yaml
# fluentd configuration
<source>
  @type tail
  path /var/log/dolt/dolt.log
  pos_file /var/log/dolt.log.pos
  tag dolt.logs
  format json
</source>

<match dolt.logs>
  @type elasticsearch
  host elasticsearch.internal
  port 9200
</match>
```

### 6.3 Dashboards

**Grafana Dashboard Panels:**

1. **Query Performance**
   - Queries per second
   - p50/p90/p99 latency
   - Error rate

2. **Connection Health**
   - Active connections
   - Connection pool usage
   - Rejected connections

3. **Storage Metrics**
   - Total storage used
   - Chunk cache hit rate
   - GC frequency

4. **Replication Status**
   - Replica lag
   - Sync status
   - Failover events

---

## 7. Security Hardening

### 7.1 Authentication

**Database Users:**
```sql
-- Create user with limited privileges
CREATE USER 'readonly'@'%' IDENTIFIED BY 'secure_password';
GRANT SELECT ON *.* TO 'readonly'@'%';

-- Create admin user
CREATE USER 'admin'@'10.0.0.%' IDENTIFIED BY 'admin_password';
GRANT ALL PRIVILEGES ON *.* TO 'admin'@'10.0.0.%';
```

**TLS Configuration:**
```yaml
# config.yaml
sql_server:
  tls: required
  cert: /etc/dolt/certs/server.crt
  key: /etc/dolt/certs/server.key
  ca: /etc/dolt/certs/ca.crt
```

### 7.2 Network Security

**VPC Configuration:**
```
┌─────────────────────────────────────────────────────┐
│                    Public Subnet                     │
│  ┌─────────────┐                                    │
│  │   Bastion   │                                    │
│  │   Host      │                                    │
│  └──────┬──────┘                                    │
└─────────│────────────────────────────────────────────┘
          │ SSH (22)
┌─────────▼────────────────────────────────────────────┐
│                   Private Subnet                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │  Dolt-1     │  │  Dolt-2     │  │  Dolt-3     │  │
│  │  (3306)     │  │  (3306)     │  │  (3306)     │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Security Group Rules:**
```yaml
# terraform security group
resource "aws_security_group" "dolt" {
  ingress {
    from_port   = 3306
    to_port     = 3306
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/8"]  # VPC only
  }

  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/32"]  # Bastion only
  }

  egress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]  # S3 access
  }
}
```

### 7.3 Audit Logging

**Enable Audit Logs:**
```sql
-- Log all queries
SET GLOBAL general_log = 'ON';
SET GLOBAL general_log_file = '/var/log/dolt/audit.log';

-- Log specific events
CALL dolt_config('--global', '--add', 'audit.log', 'true');
```

---

## 8. Multi-tenant Deployments

### 8.1 Tenant Isolation

**Database-per-Tenant:**
```
Tenant A                    Tenant B                    Tenant C
┌─────────────┐            ┌─────────────┐            ┌─────────────┐
│  DB-A1      │            │  DB-B1      │            │  DB-C1      │
│  DB-A2      │            │  DB-B2      │            │  DB-C2      │
└──────┬──────┘            └──────┬──────┘            └──────┬──────┘
       │                         │                         │
       └─────────────────────────┴─────────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │    Tenant Router        │
                    │  (routes by subdomain)  │
                    └─────────────────────────┘
```

**Routing Configuration:**
```yaml
# tenant-router.yaml
tenants:
  - subdomain: acme
    databases: [acme-main, acme-analytics]
    endpoint: dolt-acm.internal
  - subdomain: globex
    databases: [globex-main]
    endpoint: dolt-glb.internal
```

### 8.2 Resource Quotas

```sql
-- Set per-tenant limits
CALL dolt_config('--local', '--add', 'limits.max_connections', '50');
CALL dolt_config('--local', '--add', 'limits.max_storage_gb', '100');
CALL dolt_config('--local', '--add', 'limits.max_branches', '20');
```

---

## 9. Upgrade Strategy

### 9.1 Rolling Upgrade

```
Phase 1: Upgrade replicas
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Primary    │  │  Replica-1  │  │  Replica-2  │
│  v1.0.0     │  │  v1.0.0     │  │  v1.0.0     │
│  ●          │  │  ○          │  │  ○          │
└─────────────┘  └──────┬──────┘  └──────┬──────┘
                        │                 │
                        ▼                 ▼
                  Upgrade to          Upgrade to
                  v1.1.0              v1.1.0

Phase 2: Failover and upgrade primary
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Primary    │  │  Replica-1  │  │  Replica-2  │
│  v1.0.0     │  │  v1.1.0     │  │  v1.1.0     │
│  ●          │  │  ○          │  │  ○          │
└──────┬──────┘  └─────────────┘  └─────────────┘
       │
       │ Failover
       ▼

┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Replica-1  │  │  Primary    │  │  Replica-2  │
│  v1.1.0     │  │  v1.0.0     │  │  v1.1.0     │
│  ●          │  │  ○          │  │  ○          │
└─────────────┘  └──────┬──────┘  └─────────────┘
                        │
                        ▼
                  Upgrade to
                  v1.1.0
```

---

## 10. Cost Optimization

### 10.1 Storage Tiering

```yaml
# S3 lifecycle policy
Rules:
  - ID: MoveToGlacier
    Status: Enabled
    Transitions:
      - Days: 30
        StorageClass: STANDARD_IA
      - Days: 90
        StorageClass: GLACIER

  - ID: ExpireOldBackups
    Status: Enabled
    Expiration:
      Days: 365
```

### 10.2 Compute Rightsizing

**Analyze Resource Usage:**
```sql
-- Check average connections
SELECT AVG(active_connections) FROM dolt_metrics;

-- Check query patterns
SELECT
    HOUR(timestamp) as hour,
    COUNT(*) as queries
FROM dolt_query_log
GROUP BY HOUR(timestamp);
```

---

## Summary

Production Dolt deployments require:

1. **High Availability** - Replicas, failover, health checks
2. **Scalability** - Read replicas, sharding, connection pooling
3. **Performance** - Caching, indexing, query optimization
4. **Backup/Recovery** - Regular backups, tested recovery procedures
5. **Monitoring** - Metrics, logging, alerting
6. **Security** - TLS, authentication, audit logging
7. **Multi-tenancy** - Isolation, quotas, routing

---

*This document complements the rust-revision.md which covers implementation details for reproducing Dolt in Rust.*
