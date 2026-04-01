---
title: "GoatPlatform Production Deployment"
subtitle: "Deployment patterns, monitoring, and scaling for real-time sync databases"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.goatplatform
related: rust-revision.md
---

# Production-Grade GoatPlatform

## Overview

This document covers production deployment of goatdb-based applications - scaling sync servers, monitoring, and operational best practices.

## Part 1: Sync Server Deployment

### Single Server Deployment

```yaml
# docker-compose.yml

version: '3.8'

services:
  sync-server:
    image: goatdb/sync-server:latest
    ports:
      - "7687:7687"  # WebSocket
      - "9090:9090"  # Metrics
    environment:
      - RUST_LOG=info
      - DATA_PATH=/data
      - MAX_CONNECTIONS=1000
    volumes:
      - sync-data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9090/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  # Backup service
  backup:
    image: goatdb/backup:latest
    environment:
      - BACKUP_PATH=/backup
      - DATA_PATH=/data
    volumes:
      - sync-data:/data
      - backup-storage:/backup
    restart: unless-stopped

volumes:
  sync-data:
  backup-storage:
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sync-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: sync-server
  template:
    metadata:
      labels:
        app: sync-server
    spec:
      containers:
        - name: sync-server
          image: goatdb/sync-server:latest
          ports:
            - name: ws
              containerPort: 7687
            - name: metrics
              containerPort: 9090
          env:
            - name: RUST_LOG
              value: "info"
            - name: DATA_PATH
              value: "/data"
            - name: MAX_CONNECTIONS
              value: "5000"
          resources:
            requests:
              cpu: "500m"
              memory: "512Mi"
            limits:
              cpu: "2000m"
              memory: "2Gi"
          volumeMounts:
            - name: data
              mountPath: /data
          livenessProbe:
            httpGet:
              path: /health/live
              port: 9090
            initialDelaySeconds: 30
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 9090
            initialDelaySeconds: 10
            periodSeconds: 10
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: sync-server-data
---
apiVersion: v1
kind: Service
metadata:
  name: sync-server
spec:
  selector:
    app: sync-server
  ports:
    - name: ws
      port: 7687
      targetPort: 7687
  type: ClusterIP
---
apiVersion: v1
kind: Service
metadata:
  name: sync-server-external
spec:
  selector:
    app: sync-server
  ports:
    - name: ws
      port: 443
      targetPort: 7687
  type: LoadBalancer
```

## Part 2: Monitoring

### Prometheus Metrics

```rust
use prometheus::{register_counter, register_gauge, register_histogram};

pub struct Metrics {
    connections_total: prometheus::IntCounter,
    connections_active: prometheus::IntGauge,
    sync_requests_total: prometheus::IntCounter,
    sync_latency: prometheus::Histogram,
    conflicts_total: prometheus::IntCounter,
    bytes_sent: prometheus::IntCounter,
    bytes_received: prometheus::IntCounter,
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        Ok(Self {
            connections_total: register_counter!(
                "goatdb_connections_total",
                "Total number of connections"
            )?,
            connections_active: register_gauge!(
                "goatdb_connections_active",
                "Current active connections"
            )?,
            sync_requests_total: register_counter!(
                "goatdb_sync_requests_total",
                "Total sync requests"
            )?,
            sync_latency: register_histogram!(
                "goatdb_sync_latency_seconds",
                "Sync request latency",
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
            )?,
            conflicts_total: register_counter!(
                "goatdb_conflicts_total",
                "Total conflicts detected"
            )?,
            bytes_sent: register_counter!(
                "goatdb_bytes_sent_total",
                "Total bytes sent to clients"
            )?,
            bytes_received: register_counter!(
                "goatdb_bytes_received_total",
                "Total bytes received from clients"
            )?,
        })
    }

    pub fn record_connection(&self) {
        self.connections_total.inc();
        self.connections_active.inc();
    }

    pub fn record_disconnect(&self) {
        self.connections_active.dec();
    }

    pub fn record_sync(&self, latency: f64) {
        self.sync_requests_total.inc();
        self.sync_latency.observe(latency);
    }

    pub fn record_conflict(&self) {
        self.conflicts_total.inc();
    }

    pub fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.inc_by(bytes);
    }

    pub fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received.inc_by(bytes);
    }
}
```

### Prometheus Alert Rules

```yaml
# prometheus-alerts.yml

groups:
  - name: goatdb
    rules:
      # Availability
      - alert: GoatDBSyncServerDown
        expr: up{job="goatdb"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "GoatDB sync server is down"
          description: "Instance {{ $labels.instance }} has been down for 5 minutes"

      - alert: GoatDBHighDisconnectRate
        expr: rate(goatdb_connections_total[5m]) - rate(goatdb_connections_active[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High disconnection rate"
          description: "More than 10 disconnections per second"

      # Performance
      - alert: GoatDBHighSyncLatency
        expr: histogram_quantile(0.99, rate(goatdb_sync_latency_seconds_bucket[5m])) > 1
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High sync latency"
          description: "P99 sync latency is above 1 second"

      - alert: GoatDBHighConflictRate
        expr: rate(goatdb_conflicts_total[5m]) > 100
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High conflict rate"
          description: "More than 100 conflicts per second"

      # Resource
      - alert: GoatDBHighMemoryUsage
        expr: process_resident_memory_bytes{job="goatdb"} > 1073741824
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage"
          description: "Memory usage above 1GB"
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "GoatDB Overview",
    "panels": [
      {
        "title": "Active Connections",
        "type": "graph",
        "targets": [
          {
            "expr": "goatdb_connections_active",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Sync Requests per Second",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(goatdb_sync_requests_total[1m])",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Sync Latency (P99)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.99, rate(goatdb_sync_latency_seconds_bucket[1m]))",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Conflicts per Minute",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(goatdb_conflicts_total[1m]) * 60",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Network Throughput",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(goatdb_bytes_sent_total[1m])",
            "legendFormat": "{{ instance }} - sent"
          },
          {
            "expr": "rate(goatdb_bytes_received_total[1m])",
            "legendFormat": "{{ instance }} - received"
          }
        ]
      }
    ]
  }
}
```

## Part 3: Scaling Strategies

### Horizontal Scaling with Sharding

```rust
use consistent_hash::ConsistentHash;

pub struct ShardedSyncServer {
    shards: Vec<Arc<SyncServer>>,
    hash_ring: ConsistentHash<String>,
}

impl ShardedSyncServer {
    pub fn new(num_shards: usize) -> Self {
        let mut hash_ring = ConsistentHash::new();
        let mut shards = Vec::new();

        for i in 0..num_shards {
            let shard_id = format!("shard-{}", i);
            hash_ring.add(shard_id.clone());

            let server = SyncServer::new();
            shards.push(Arc::new(server));
        }

        Self { shards, hash_ring }
    }

    fn get_shard(&self, tenant_id: &str) -> Arc<SyncServer> {
        let shard_id = self.hash_ring.get(&tenant_id.to_string()).unwrap();
        let index = shard_id.strip_prefix("shard-").unwrap().parse().unwrap();
        self.shards[index].clone()
    }

    pub async fn handle_client(
        &self,
        tenant_id: &str,
        ws: WebSocket,
    ) {
        let shard = self.get_shard(tenant_id);
        shard.handle_connection(ws).await;
    }
}
```

### Read Replica Pattern

```rust
pub struct SyncServerWithReplicas {
    primary: Arc<SyncServer>,
    replicas: Vec<Arc<SyncServer>>,
    replica_index: AtomicUsize,
}

impl SyncServerWithReplicas {
    pub fn new(primary: Arc<SyncServer>, replicas: Vec<Arc<SyncServer>>) -> Self {
        Self {
            primary,
            replicas,
            replica_index: AtomicUsize::new(0),
        }
    }

    // Writes go to primary
    pub async fn handle_write(&self, change: Change) -> Result<(), SyncError> {
        self.primary.apply_change(change).await
    }

    // Reads can go to any replica (round-robin)
    pub fn get_replica(&self) -> Arc<SyncServer> {
        let index = self.replica_index.fetch_add(1, Ordering::Relaxed);
        let replica_index = index % self.replicas.len();
        self.replicas[replica_index].clone()
    }

    pub async fn handle_read(&self, table: &str, row_id: &str) -> Result<Option<Row>, SyncError> {
        let replica = self.get_replica();
        replica.read(table, row_id).await
    }
}
```

## Part 4: Backup and Recovery

### Online Backup

```rust
use std::process::Command;

pub struct BackupManager {
    backup_path: String,
    retention_days: u32,
}

impl BackupManager {
    pub fn new(backup_path: &str, retention_days: u32) -> Self {
        Self {
            backup_path: backup_path.to_string(),
            retention_days,
        }
    }

    pub async fn create_backup(&self, data_path: &str) -> Result<String, BackupError> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("backup_{}", timestamp);
        let backup_dir = format!("{}/{}", self.backup_path, backup_name);

        // Create backup directory
        std::fs::create_dir_all(&backup_dir)?;

        // Copy data files (using rsync for efficiency)
        let output = Command::new("rsync")
            .args([
                "-av",
                "--delete",
                &format!("{}/", data_path),
                &format!("{}/data/", backup_dir),
            ])
            .output()?;

        if !output.status.success() {
            return Err(BackupError::CopyFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        // Create metadata
        let metadata = BackupMetadata {
            name: backup_name,
            created_at: chrono::Utc::now(),
            data_size: Self::get_directory_size(&format!("{}/data", backup_dir))?,
        };

        // Save metadata
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(format!("{}/metadata.json", backup_dir), metadata_json)?;

        Ok(backup_name)
    }

    pub fn cleanup_old_backups(&self) -> Result<usize, BackupError> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(self.retention_days as i64);
        let mut removed = 0;

        for entry in std::fs::read_dir(&self.backup_path)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let metadata_path = path.join("metadata.json");
            if !metadata_path.exists() {
                continue;
            }

            let metadata: BackupMetadata = serde_json::from_slice(
                &std::fs::read(&metadata_path)?
            )?;

            if metadata.created_at < cutoff {
                std::fs::remove_dir_all(&path)?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    fn get_directory_size(path: &str) -> Result<u64, std::io::Error> {
        let mut total = 0;
        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupMetadata {
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    data_size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Copy failed: {0}")]
    CopyFailed(String),
}
```

---

*This document is part of the GoatPlatform exploration series. See [exploration.md](./exploration.md) for the complete index.*
