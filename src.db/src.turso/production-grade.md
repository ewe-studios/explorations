---
title: "Production-Grade Turso/libSQL"
subtitle: "Deployment patterns, monitoring, backup, and high availability"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.turso
explored_at: 2026-03-28
related: rust-revision.md, 03-consensus-replication-deep-dive.md
---

# Production-Grade Turso/libSQL

## Overview

This document covers deploying libSQL-compatible embedded replica systems to production, including infrastructure setup, monitoring, backup strategies, and high-availability configurations.

## Part 1: Deployment Architecture

### Reference Architecture

```
                            ┌─────────────────┐
                            │   Cloudflare    │
                            │     Worker      │
                            │  (Edge Cache)   │
                            └────────┬────────┘
                                     │
         ┌───────────────────────────┼───────────────────────────┐
         │                           │                           │
         ▼                           ▼                           ▼
┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│   App Server    │         │   App Server    │         │   App Server    │
│   (US-East)     │         │   (US-West)     │         │   (EU-West)     │
│                 │         │                 │         │                 │
│ ┌─────────────┐ │         │ ┌─────────────┐ │         │ ┌─────────────┐ │
│ │  Embedded   │ │         │ │  Embedded   │ │         │ │  Embedded   │ │
│ │  Replica    │ │         │ │  Replica    │ │         │ │  Replica    │ │
│ │  (SQLite)   │ │         │ │  (SQLite)   │ │         │ │  (SQLite)   │ │
│ └─────────────┘ │         │ └─────────────┘ │         │ └─────────────┘ │
└────────┬────────┘         └────────┬────────┘         └────────┬────────┘
         │                           │                           │
         └───────────────────────────┼───────────────────────────┘
                                     │
                                     ▼
                            ┌─────────────────┐
                            │   Primary DB    │
                            │   (Turso/libSQL │
                            │    or custom)   │
                            │                 │
                            │ ┌─────────────┐ │
                            │ │     WAL     │ │
                            │ │   Engine    │ │
                            │ └─────────────┘ │
                            └────────┬────────┘
                                     │
                            ┌─────────────────┐
                            │   S3 Bucket     │
                            │  (WAL Backup)   │
                            └─────────────────┘
```

### Component Sizing

| Component | Small | Medium | Large |
|-----------|-------|--------|-------|
| **Embedded Replica** | | | |
| RAM | 256 MB | 1 GB | 4 GB |
| Disk | 10 GB | 50 GB | 200 GB |
| Max DB Size | 1 GB | 10 GB | 50 GB |
| **Primary** | | | |
| RAM | 2 GB | 8 GB | 32 GB |
| CPU | 2 vCPU | 4 vCPU | 16 vCPU |
| Disk | 50 GB SSD | 200 GB SSD | 1 TB NVMe |
| Max Write Throughput | 1K ops/sec | 10K ops/sec | 100K ops/sec |

### Infrastructure as Code (Pulumi Example)

```typescript
import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";

// Primary database cluster
const primaryVpc = new awsx.ec2.Vpc("libsql-primary-vpc", {
  cidrBlock: "10.0.0.0/16",
  numberOfAvailabilityZones: 3,
});

// RDS instance (or self-managed)
const primaryDb = new aws.rds.Instance("libsql-primary", {
  instanceClass: "db.r6g.large",
  engine: "custom",  // Custom engine for libSQL
  allocatedStorage: 200,
  storageType: "gp3",
  vpcSecurityGroupIds: [primarySecurityGroup.id],
  dbSubnetGroupName: primarySubnetGroup.name,
  backupRetentionPeriod: 30,
  multiAz: true,
});

// S3 bucket for WAL backup
const walBucket = new aws.s3.Bucket("libsql-wal-backup", {
  versioning: {
    enabled: true,
  },
  lifecycleRules: [{
    expiration: {
      days: 7,  // Keep WAL for 7 days
    },
  }],
});

// Application servers with embedded replicas
const appCluster = new awsx.ec2.AutoScalingGroup("libsql-app", {
  vpc: primaryVpc,
  desiredCapacity: 3,
  minSize: 2,
  maxSize: 10,
  instanceType: "t4g.medium",
  launchTemplate: {
    userData: `#!/bin/bash
      # Mount EBS volume for embedded replica
      mkfs -t xfs /dev/nvme1n1
      mount /dev/nvme1n1 /var/lib/libsql

      # Start application
      /opt/app/start.sh
    `,
  },
});

// CloudWatch alarms for replication lag
const replicationLagAlarm = new aws.cloudwatch.MetricAlarm("replication-lag", {
  comparisonOperator: "GreaterThanThreshold",
  evaluationPeriods: 3,
  metricName: "ReplicationLagSeconds",
  namespace: "LibSQL",
  period: 60,
  statistic: "Average",
  threshold: 60,  // Alert if lag > 60 seconds
  alarmActions: [snsTopic.arn],
});
```

## Part 2: Monitoring and Observability

### Key Metrics to Track

```yaml
# Prometheus metrics configuration

# Replica metrics
libsql_replica:
  - current_frame_offset: "Current WAL position"
  - primary_frame_offset: "Primary WAL position (from last sync)"
  - replication_lag_seconds: "Time-based lag"
  - replication_lag_frames: "Frame-based lag"
  - sync_duration_seconds: "Time to complete sync"
  - sync_errors_total: "Total sync failures"

# Query metrics
libsql_queries:
  - total: "Total queries executed"
  - by_type: "SELECT vs WRITE"
  - latency_seconds: "Query latency histogram"
  - errors_total: "Failed queries"

# Connection metrics
libsql_connections:
  - active: "Current active connections"
  - waiting: "Connections waiting for lock"
  - lock_wait_seconds: "Time spent waiting for locks"

# Storage metrics
libsql_storage:
  - database_size_bytes: "Total database size"
  - wal_size_bytes: "WAL file size"
  - page_cache_hits: "Cache hit count"
  - page_cache_misses: "Cache miss count"
  - checkpoint_duration_seconds: "Checkpoint time"
```

### Dashboard Configuration (Grafana)

```json
{
  "dashboard": {
    "title": "libSQL Embedded Replicas",
    "panels": [
      {
        "title": "Replication Lag",
        "type": "graph",
        "targets": [
          {
            "expr": "libsql_replica_replication_lag_seconds",
            "legendFormat": "{{replica_id}}"
          }
        ],
        "thresholds": [
          { "value": 30, "color": "yellow" },
          { "value": 60, "color": "red" }
        ]
      },
      {
        "title": "Query Latency (p99)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.99, rate(libsql_queries_latency_seconds_bucket[5m]))"
          }
        ]
      },
      {
        "title": "Sync Errors",
        "type": "stat",
        "targets": [
          {
            "expr": "rate(libsql_replica_sync_errors_total[5m])"
          }
        ],
        "thresholds": [
          { "value": 0.1, "color": "red" }
        ]
      },
      {
        "title": "Database Size",
        "type": "graph",
        "targets": [
          {
            "expr": "libsql_storage_database_size_bytes"
          }
        ]
      }
    ]
  }
}
```

### Alerting Rules

```yaml
groups:
- name: libsql_production
  rules:
  # Critical: Replica completely stopped syncing
  - alert: ReplicaSyncStopped
    expr: libsql_replica_replication_lag_seconds > 3600
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "Replica {{ $labels.replica_id }} has stopped syncing"
      description: "Replication lag is {{ $value }} seconds - data may be severely stale"

  # Warning: High replication lag
  - alert: ReplicaLagHigh
    expr: libsql_replica_replication_lag_seconds > 60
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "Replica {{ $labels.replica_id }} has high replication lag"
      description: "Replication lag is {{ $value }} seconds"

  # Critical: Sync errors
  - alert: ReplicaSyncErrors
    expr: rate(libsql_replica_sync_errors_total[5m]) > 0.1
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "Replica {{ $labels.replica_id }} is experiencing sync errors"

  # Warning: Query latency high
  - alert: QueryLatencyHigh
    expr: histogram_quantile(0.99, rate(libsql_queries_latency_seconds_bucket[5m])) > 0.5
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "Query latency is high on {{ $labels.replica_id }}"
      description: "p99 latency is {{ $value }} seconds"

  # Warning: Database growing fast
  - alert: DatabaseGrowthRate
    expr: rate(libsql_storage_database_size_bytes[1h]) > 1073741824  # 1GB/hour
    for: 1h
    labels:
      severity: warning
    annotations:
      summary: "Database {{ $labels.replica_id }} is growing rapidly"
      description: "Growing at {{ $value | humanize }} bytes/hour"

  # Critical: Disk space low
  - alert: DiskSpaceLow
    expr: node_filesystem_avail_bytes{mountpoint="/var/lib/libsql"} / node_filesystem_size_bytes{mountpoint="/var/lib/libsql"} < 0.1
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "Disk space is critically low on {{ $labels.instance }}"
```

## Part 3: Backup and Recovery

### Backup Strategy

```
┌─────────────────────────────────────────────────────────────┐
│                    Backup Schedule                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Primary Database:                                           │
│  ├─ Continuous: WAL frames to S3 (every commit)             │
│  ├─ Hourly: Checkpoint + full snapshot                      │
│  └─ Daily: Full backup with retention (30 days)             │
│                                                              │
│  Embedded Replicas:                                          │
│  ├─ On-demand: Snapshot before major deploys                │
│  └─ Weekly: Full backup for disaster recovery               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Backup Implementation

```rust
use aws_sdk_s3::Client as S3Client;
use chrono::{Utc, Datelike};

pub struct BackupManager {
    s3_client: S3Client,
    bucket: String,
    db_path: String,
}

impl BackupManager {
    pub fn new(s3_client: S3Client, bucket: String, db_path: String) -> Self {
        Self {
            s3_client,
            bucket,
            db_path,
        }
    }

    /// Create full database backup
    pub async fn create_full_backup(&self) -> Result<BackupInfo, BackupError> {
        let now = Utc::now();
        let key = format!(
            "backups/full/{year}/{month:02}/{day:02}/{timestamp}.backup.gz",
            year = now.year(),
            month = now.month(),
            day = now.day(),
            timestamp = now.timestamp()
        );

        // Create snapshot (copy database file)
        let snapshot_path = format!("{}.snapshot", self.db_path);
        std::fs::copy(&self.db_path, &snapshot_path)?;

        // Compress
        let compressed_path = format!("{}.gz", snapshot_path);
        compress_file(&snapshot_path, &compressed_path)?;

        // Upload to S3
        let data = std::fs::read(&compressed_path)?;
        self.s3_client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.into())
            .send()
            .await?;

        // Cleanup local files
        std::fs::remove_file(&snapshot_path)?;
        std::fs::remove_file(&compressed_path)?;

        Ok(BackupInfo {
            key,
            size_bytes: data.len() as u64,
            created_at: now,
            backup_type: BackupType::Full,
        })
    }

    /// Backup WAL frames incrementally
    pub async fn backup_wal(&self, wal_path: &str, start_offset: u64) -> Result<u64, BackupError> {
        let now = Utc::now();
        let key = format!(
            "backups/wal/{year}/{month:02}/{day:02}/{timestamp}_{offset}.wal",
            year = now.year(),
            month = now.month(),
            day = now.day(),
            timestamp = now.timestamp(),
            offset = start_offset
        );

        // Read WAL file from offset
        let wal_data = std::fs::read(wal_path)?;

        // Upload to S3
        self.s3_client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(wal_data.into())
            .send()
            .await?;

        Ok(wal_data.len() as u64)
    }

    /// List available backups
    pub async fn list_backups(&self) -> Result<Vec<BackupInfo>, BackupError> {
        let response = self.s3_client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix("backups/")
            .send()
            .await?;

        let backups = response
            .contents()
            .iter()
            .filter_map(|obj| {
                Some(BackupInfo {
                    key: obj.key()?.to_string(),
                    size_bytes: obj.size() as u64,
                    created_at: obj.last_modified()?.to_chrono(),
                    backup_type: if obj.key()?.contains("/full/") {
                        BackupType::Full
                    } else {
                        BackupType::Wal
                    },
                })
            })
            .collect();

        Ok(backups)
    }

    /// Restore from backup
    pub async fn restore(&self, backup_key: &str) -> Result<(), BackupError> {
        // Download from S3
        let response = self.s3_client
            .get_object()
            .bucket(&self.bucket)
            .key(backup_key)
            .send()
            .await?;

        let data = response.body.collect().await?.to_vec();

        // Decompress if needed
        let db_data = if backup_key.ends_with(".gz") {
            decompress_data(&data)?
        } else {
            data
        };

        // Write to database file
        std::fs::write(&self.db_path, &db_data)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct BackupInfo {
    pub key: String,
    pub size_bytes: u64,
    pub created_at: chrono::DateTime<Utc>,
    pub backup_type: BackupType,
}

#[derive(Debug, Clone)]
pub enum BackupType {
    Full,
    Wal,
}
```

### Recovery Procedures

```
Scenario 1: Single replica failure

1. Detect failure (health check, metrics)
2. Stop traffic to affected replica
3. Create new EC2 instance
4. Mount fresh EBS volume
5. Download latest full backup from S3
6. Apply incremental WAL backups
7. Start application with sync enabled
8. Verify sync caught up
9. Add to load balancer


Scenario 2: Primary failure

1. Detect primary failure
2. Elect new primary (manual or automated)
   a. Choose replica with least lag
   b. Promote to read-write mode
   c. Update DNS/connection strings
3. Other replicas reconfigure to sync from new primary
4. Investigate and replace failed primary


Scenario 3: Data corruption

1. Stop all writes to affected database
2. Identify corruption point (checksum failures)
3. Find last known-good backup
4. Restore backup to clean volume
5. Apply WAL frames up to corruption point (skip corrupted)
6. Verify data integrity
7. Resume normal operations
```

## Part 4: High Availability

### Multi-Region Deployment

```
┌─────────────────────────────────────────────────────────────────┐
│                     Global Traffic Management                    │
│                         (Route53 / CloudFront)                   │
└─────────────────────────┬───────────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│   US-East-1   │ │   EU-West-1   │ │  AP-Northeast │
│               │ │               │ │               │
│ ┌───────────┐ │ │ ┌───────────┐ │ │ ┌───────────┐ │
│ │  Primary  │ │ │ │ Secondary │ │ │ │ Secondary │ │
│ │  (R/W)    │ │ │ │  (R/O)    │ │ │ │  (R/O)    │ │
│ └───────────┘ │ │ └───────────┘ │ │ └───────────┘ │
│       │       │ │       │       │ │       │       │
│       │ sync  │ │       │       │ │       │       │
│       └───────┼─┼───────┘       │ │       │       │
│               │ │               │ │       │       │
│ ┌───────────┐ │ │ ┌───────────┐ │ │ ┌───────────┐ │
│ │ Embedded  │ │ │ │ Embedded  │ │ │ │ Embedded  │ │
│ │ Replicas  │ │ │ │ Replicas  │ │ │ │ Replicas  │ │
│ │  (App)    │ │ │ │  (App)    │ │ │ │  (App)    │ │
│ └───────────┘ │ │ └───────────┘ │ │ └───────────┘ │
└───────────────┘ └───────────────┘ └───────────────┘
```

### Failover Configuration

```rust
#[derive(Debug, Clone)]
pub struct FailoverConfig {
    /// Health check interval
    pub health_check_interval_secs: u64,

    /// Number of failed checks before failover
    pub failure_threshold: u32,

    /// Max acceptable replication lag for promotion
    pub max_lag_for_promotion_secs: u64,

    /// Automatic failover enabled
    pub automatic_failover: bool,

    /// Preferred promotion order (replica priority)
    pub promotion_priority: Vec<String>,
}

pub struct FailoverManager {
    config: FailoverConfig,
    primary_endpoint: String,
    replicas: Vec<ReplicaInfo>,
    current_primary_index: usize,
}

impl FailoverManager {
    pub fn new(config: FailoverConfig, primary: String, replicas: Vec<ReplicaInfo>) -> Self {
        Self {
            config,
            primary_endpoint: primary,
            replicas,
            current_primary_index: 0,
        }
    }

    /// Check health of all replicas
    pub async fn check_health(&self) -> Vec<ReplicaHealth> {
        let mut health_checks = Vec::new();

        for (i, replica) in self.replicas.iter().enumerate() {
            let health = self.check_single_replica(replica).await;
            health_checks.push(ReplicaHealth {
                index: i,
                replica_id: replica.id.clone(),
                is_healthy: health.is_healthy,
                replication_lag_secs: health.replication_lag_secs,
                last_check: Utc::now(),
            });
        }

        health_checks
    }

    async fn check_single_replica(&self, replica: &ReplicaInfo) -> HealthResult {
        // Try to connect and run simple query
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            replica.execute("SELECT 1")
        ).await;

        match result {
            Ok(Ok(_)) => HealthResult {
                is_healthy: true,
                replication_lag_secs: replica.get_lag().await.unwrap_or(u64::MAX),
            },
            _ => HealthResult {
                is_healthy: false,
                replication_lag_secs: u64::MAX,
            },
        }
    }

    /// Initiate failover
    pub async fn failover(&mut self) -> Result<String, FailoverError> {
        // Find best candidate for promotion
        let candidate = self.find_promotion_candidate().await?;

        // Promote to primary
        self.promote_replica(candidate).await?;

        // Reconfigure other replicas
        self.reconfigure_replicas(candidate).await?;

        Ok(self.replicas[candidate].id.clone())
    }

    async fn find_promotion_candidate(&self) -> Result<usize, FailoverError> {
        // Check replicas in priority order
        for &priority_idx in &self.config.promotion_priority {
            if priority_idx >= self.replicas.len() {
                continue;
            }

            let replica = &self.replicas[priority_idx];
            let lag = replica.get_lag().await?;

            if lag <= self.config.max_lag_for_promotion_secs {
                return Ok(priority_idx);
            }
        }

        Err(FailoverError::NoEligibleCandidate)
    }

    async fn promote_replica(&mut self, index: usize) -> Result<(), FailoverError> {
        // Tell replica to accept writes
        self.replicas[index].set_read_write(true).await?;

        // Update current primary index
        self.current_primary_index = index;

        Ok(())
    }

    async fn reconfigure_replicas(&self, new_primary_index: usize) -> Result<(), FailoverError> {
        let new_primary_url = self.replicas[new_primary_index].sync_url.clone();

        for (i, replica) in self.replicas.iter().enumerate() {
            if i == new_primary_index {
                continue;  // Skip the new primary
            }

            replica.reconfigure_sync(&new_primary_url).await?;
        }

        Ok(())
    }
}
```

## Part 5: Performance Tuning

### SQLite Configuration

```sql
-- Optimize for embedded replica workload

-- WAL mode (required for replication)
PRAGMA journal_mode = WAL;

-- Synchronous setting (trade durability for performance)
PRAGMA synchronous = NORMAL;  -- or FULL for maximum safety

-- Cache size (adjust based on available RAM)
PRAGMA cache_size = -64000;  -- 64 MB negative = KB

-- Memory-mapped I/O
PRAGMA mmap_size = 268435456;  -- 256 MB

-- Busy timeout (how long to wait for locks)
PRAGMA busy_timeout = 5000;  -- 5 seconds

-- Checkpoint threshold (auto-checkpoint when WAL exceeds this)
PRAGMA wal_autocheckpoint = 1000;  -- 1000 pages

-- Optimize query planning
PRAGMA optimize;

-- Foreign keys (enable if using)
PRAGMA foreign_keys = ON;
```

### Application-Level Optimizations

```rust
// Use connection pooling
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

pub struct DatabasePool {
    pool: Pool<SqliteConnectionManager>,
}

impl DatabasePool {
    pub fn new(db_path: &str, max_connections: u32) -> Result<Self, Error> {
        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::builder()
            .max_size(max_connections)
            .min_idle(Some(2))
            .connection_timeout(Duration::from_secs(30))
            .build(manager)?;

        Ok(Self { pool })
    }

    pub fn get_conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, Error> {
        Ok(self.pool.get()?)
    }

    // Batch operations
    pub fn batch_insert(&self, table: &str, rows: &[Vec<Value>]) -> Result<(), Error> {
        let conn = self.get_conn()?;

        conn.execute("BEGIN")?;

        for row in rows {
            // Build and execute insert
            // ...
        }

        conn.execute("COMMIT")?;
        Ok(())
    }
}

// Prepared statement caching
use std::collections::HashMap;
use rusqlite::{Connection, Statement};

pub struct CachedStatements<'a> {
    conn: &'a Connection,
    cache: HashMap<String, Statement<'a>>,
}

impl<'a> CachedStatements<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            cache: HashMap::new(),
        }
    }

    pub fn prepare(&mut self, sql: &str) -> Result<&Statement, rusqlite::Error> {
        use std::collections::hash_map::Entry;

        match self.cache.entry(sql.to_string()) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let stmt = self.conn.prepare(sql)?;
                Ok(entry.insert(stmt))
            }
        }
    }
}
```

---

*This document is part of the Turso/libSQL exploration series. See [exploration.md](./exploration.md) for the complete index.*
