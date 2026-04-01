---
title: "OrbitingHail Production Deployment"
subtitle: "Deploying SQLSync servers, monitoring, and scaling patterns"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.orbitinghail
related: rust-revision.md
---

# Production-Grade OrbitingHail

## Overview

This document covers production deployment of SQLSync-based applications - sync server deployment, monitoring, and scaling strategies.

## Part 1: Sync Server Deployment

### Docker Deployment

```yaml
# docker-compose.yml

version: '3.8'

services:
  sync-server:
    image: orbitinghail/sync-server:latest
    ports:
      - "9229:9229"  # WebSocket sync port
      - "9090:9090"  # Metrics port
    environment:
      - RUST_LOG=info
      - DATABASE_URL=postgres://user:pass@postgres:5432/sqlsync
      - MAX_CONNECTIONS=5000
      - SYNC_BATCH_SIZE=100
    depends_on:
      - postgres
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9090/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=sqlsync
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=pass
    volumes:
      - postgres-data:/var/lib/postgresql/data
    restart: unless-stopped

volumes:
  postgres-data:
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
          image: orbitinghail/sync-server:latest
          ports:
            - name: ws
              containerPort: 9229
            - name: metrics
              containerPort: 9090
          env:
            - name: RUST_LOG
              value: "info"
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: db-secret
                  key: url
            - name: MAX_CONNECTIONS
              value: "5000"
          resources:
            requests:
              cpu: "250m"
              memory: "256Mi"
            limits:
              cpu: "1000m"
              memory: "1Gi"
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
      port: 9229
      targetPort: 9229
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
      targetPort: 9229
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
    changes_uploaded: prometheus::IntCounter,
    changes_downloaded: prometheus::IntCounter,
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        Ok(Self {
            connections_total: register_counter!(
                "sqlsync_connections_total",
                "Total WebSocket connections"
            )?,
            connections_active: register_gauge!(
                "sqlsync_connections_active",
                "Active WebSocket connections"
            )?,
            sync_requests_total: register_counter!(
                "sqlsync_sync_requests_total",
                "Total sync requests"
            )?,
            sync_latency: register_histogram!(
                "sqlsync_sync_latency_seconds",
                "Sync request latency",
                vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
            )?,
            conflicts_total: register_counter!(
                "sqlsync_conflicts_total",
                "Total conflicts resolved"
            )?,
            changes_uploaded: register_counter!(
                "sqlsync_changes_uploaded_total",
                "Changes uploaded by clients"
            )?,
            changes_downloaded: register_counter!(
                "sqlsync_changes_downloaded_total",
                "Changes downloaded by clients"
            )?,
        })
    }
}
```

### Alert Rules

```yaml
# prometheus-alerts.yml

groups:
  - name: sqlsync
    rules:
      - alert: SQLSyncServerDown
        expr: up{job="sqlsync"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "SQLSync server is down"

      - alert: SQLSyncHighSyncLatency
        expr: histogram_quantile(0.99, rate(sqlsync_sync_latency_seconds_bucket[5m])) > 2
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High sync latency (P99 > 2s)"

      - alert: SQLSyncHighConflictRate
        expr: rate(sqlsync_conflicts_total[5m]) > 50
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High conflict rate"

      - alert: SQLSyncConnectionSaturation
        expr: sqlsync_connections_active / sqlsync_connections_max > 0.9
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Connection pool near capacity"
```

## Part 3: Scaling Strategies

### Horizontal Scaling with Partitioning

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

            shards.push(Arc::new(SyncServer::new()));
        }

        Self { shards, hash_ring }
    }

    fn get_shard_for_user(&self, user_id: &str) -> Arc<SyncServer> {
        let shard_id = self.hash_ring.get(&user_id.to_string()).unwrap();
        let index = shard_id.strip_prefix("shard-").unwrap().parse().unwrap();
        self.shards[index].clone()
    }

    pub async fn handle_sync(
        &self,
        user_id: &str,
        request: SyncRequest,
    ) -> Result<SyncResponse, SyncError> {
        let shard = self.get_shard_for_user(user_id);
        shard.process_sync(request).await
    }
}
```

### Connection Multiplexing

```rust
use tokio::sync::broadcast;

pub struct ConnectionPool {
    connections: Arc<RwLock<HashMap<String, broadcast::Sender<Message>>>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_connection(
        &self,
        client_id: &str,
        tx: broadcast::Sender<Message>,
    ) {
        self.connections.write().await.insert(client_id.to_string(), tx);
    }

    pub async fn remove_connection(&self, client_id: &str) {
        self.connections.write().await.remove(client_id);
    }

    pub async fn broadcast_to_client(
        &self,
        client_id: &str,
        message: Message,
    ) -> Result<(), BroadcastError> {
        if let Some(tx) = self.connections.read().await.get(client_id) {
            tx.send(message)?;
            Ok(())
        } else {
            Err(BroadcastError::ClientNotFound)
        }
    }

    pub async fn broadcast_changes(
        &self,
        interested_clients: &[String],
        changes: Vec<ChangeRecord>,
    ) -> Result<usize, BroadcastError> {
        let message = Message::Text(serde_json::to_string(&changes)?);
        let mut sent = 0;

        for client_id in interested_clients {
            if self.broadcast_to_client(client_id, message.clone()).await.is_ok() {
                sent += 1;
            }
        }

        Ok(sent)
    }
}
```

## Part 4: Backup and Recovery

### Database Backup

```bash
#!/bin/bash
# PostgreSQL backup for SQLSync server

BACKUP_DIR="/backup/sqlsync"
RETENTION_DAYS=7
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup
pg_dump $DATABASE_URL | gzip > $BACKUP_DIR/sqlsync_$DATE.sql.gz

# Verify backup
if gzip -t $BACKUP_DIR/sqlsync_$DATE.sql.gz; then
    echo "Backup verified: sqlsync_$DATE.sql.gz"
else
    echo "Backup verification failed!"
    exit 1
fi

# Clean old backups
find $BACKUP_DIR -name "sqlsync_*.sql.gz" -mtime +$RETENTION_DAYS -delete

echo "Backup complete"
```

### Point-in-Time Recovery

```sql
-- Enable WAL archiving in postgresql.conf
-- wal_level = replica
-- archive_mode = on
-- archive_command = 'cp %p /backup/wal/%f'

-- Restore to specific point in time
-- In recovery.conf:
-- restore_command = 'cp /backup/wal/%f %p'
-- recovery_target_time = '2024-01-15 10:30:00'
-- recovery_target_action = 'promote'
```

---

*This document is part of the OrbitingHail exploration series. See [exploration.md](./exploration.md) for the complete index.*
