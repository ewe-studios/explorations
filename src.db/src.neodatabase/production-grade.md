---
title: "Neodatabase Production Deployment"
subtitle: "Neo4j clustering, monitoring, backup strategies, and production patterns"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.neodatabase
related: rust-revision.md
---

# Production-Grade Neodatabase

## Overview

This document covers production deployment of Neo4j - clustering with Causal Clustering, monitoring, backup strategies, and operational best practices.

## Part 1: Causal Clustering

### Cluster Architecture

```
Neo4j Causal Clustering (3-node minimum):

┌─────────────────────────────────────────────────────────┐
│                  Neo4j Cluster                           │
│                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │   Core 1    │  │   Core 2    │  │   Core 3    │      │
│  │  (Leader)   │  │  (Follower) │  │  (Follower) │      │
│  ├─────────────┤  ├─────────────┤  ├─────────────┤      │
│  │ - Raft log  │  │ - Raft log  │  │ - Raft log  │      │
│  │ - Full data │  │ - Full data │  │ - Full data │      │
│  │ - Writes    │  │ - Reads     │  │ - Reads     │      │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘      │
│         │                │                │             │
│         └────────────────┼────────────────┘             │
│                          │                              │
│              Raft Consensus Protocol                    │
│              (election, log replication)                │
└───────────────────────────────────────────────────────────┘

Discovery:
- All cores must discover each other at startup
- Use DNS, Kubernetes, or static list
- Minimum 3 cores for fault tolerance (F=1)

Fault Tolerance Formula:
- Need 2F+1 cores to tolerate F failures
- 3 cores: tolerate 1 failure
- 5 cores: tolerate 2 failures
```

### Cluster Configuration

```yaml
# neo4j.conf - Core Node Configuration

# Cluster discovery
dbms.cluster.enabled=true
dbms.cluster.minimum_core_cluster_size_at_formation=3
dbms.cluster.minimum_core_cluster_size_at_runtime=2

# Discovery mechanism (choose one)

# Option 1: Static list
dbms.discovery.advertised_address=core1.neo4j.local:5000
dbms.discovery.initial_members=core1.neo4j.local:5000,core2.neo4j.local:5000,core3.neo4j.local:5000

# Option 2: DNS
dbms.discovery.type=dns
dbms.discovery.dns.name=neo4j-core.default.svc.cluster.local

# Network addresses
dbms.default_listen_address=0.0.0.0

# Cluster communication ports
dbms.connector.bolt.listen_address=:7687
dbms.connector.http.listen_address=:7474
dbms.connector.https.listen_address=:7473

# Internal cluster communication
dbms.cluster.raft.listen_address=:5000
dbms.cluster.transaction.listen_address=:6000
dbms.cluster.discovery.listen_address=:5000

# Transaction replication
dbms.tx_log.rotation.retention_policy=7 days
```

```yaml
# neo4j.conf - Read Replica Configuration

# Read replica (not part of Raft consensus)
dbms.cluster.enabled=true
dbms.cluster.discovery_type=LIST

# Connect to core cluster for discovery
dbms.cluster.discovery.advertised_address=replica1.neo4j.local:5000
dbms.discovery.initial_members=core1.neo4j.local:5000,core2.neo4j.local:5000,core3.neo4j.local:5000

# Read replicas don't participate in voting
dbms.cluster.role=read_replica

# Pull transactions from leader
dbms.tx_pull_interval=1s
```

### Kubernetes Deployment

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: neo4j-config
data:
  NEO4J_dbms_cluster_enabled: "true"
  NEO4J_dbms_cluster_minimum__core__cluster__size__at__formation: "3"
  NEO4J_dbms_cluster_minimum__core__cluster__size__at__runtime: "2"
  NEO4J_dbms_discovery_advertised__address: "neo4j-core-0.neo4j-headless.default.svc.cluster.local:5000"
  NEO4J_dbms_discovery_initial__members: "neo4j-core-0.neo4j-headless.default.svc.cluster.local:5000,neo4j-core-1.neo4j-headless.default.svc.cluster.local:5000,neo4j-core-2.neo4j-headless.default.svc.cluster.local:5000"
  NEO4J_dbms_default__listen__address: "0.0.0.0"
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: neo4j-core
spec:
  serviceName: neo4j-headless
  replicas: 3
  selector:
    matchLabels:
      app: neo4j-core
  template:
    metadata:
      labels:
        app: neo4j-core
    spec:
      containers:
        - name: neo4j
          image: neo4j:5.15-enterprise
          ports:
            - name: bolt
              containerPort: 7687
            - name: http
              containerPort: 7474
            - name: raft
              containerPort: 5000
            - name: tx
              containerPort: 6000
            - name: discovery
              containerPort: 5000
          env:
            - name: NEO4J_AUTH
              value: "neo4j/strongpassword"
            - name: NEO4J_ACCEPT_LICENSE_AGREEMENT
              value: "eval"
            - name: NEO4J_dbms_memory_heap_initial__size
              value: "2G"
            - name: NEO4J_dbms_memory_heap_max__size
              value: "4G"
            - name: NEO4J_dbms_memory_pagecache_size
              value: "2G"
          envFrom:
            - configMapRef:
                name: neo4j-config
          volumeMounts:
            - name: data
              mountPath: /data
          livenessProbe:
            httpGet:
              path: /health/live
              port: 7474
            initialDelaySeconds: 60
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 7474
            initialDelaySeconds: 30
            periodSeconds: 10
  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        accessModes: ["ReadWriteOnce"]
        storageClassName: gp3
        resources:
          requests:
            storage: 50Gi
---
apiVersion: v1
kind: Service
metadata:
  name: neo4j-headless
spec:
  clusterIP: None
  ports:
    - name: bolt
      port: 7687
      targetPort: 7687
    - name: http
      port: 7474
      targetPort: 7474
  selector:
    app: neo4j-core
---
apiVersion: v1
kind: Service
metadata:
  name: neo4j
spec:
  ports:
    - name: bolt
      port: 7687
      targetPort: 7687
  selector:
    app: neo4j-core
```

## Part 2: Monitoring

### Prometheus Metrics

```yaml
# Prometheus scrape config

scrape_configs:
  - job_name: 'neo4j'
    static_configs:
      - targets: ['neo4j-0.neo4j-headless:7474', 'neo4j-1.neo4j-headless:7474', 'neo4j-2.neo4j-headless:7474']
    metrics_path: /metrics
    scrape_interval: 15s
```

```yaml
# Prometheus Alert Rules

groups:
  - name: neo4j
    rules:
      # Cluster health
      - alert: Neo4jClusterNodeDown
        expr: absent(up{job="neo4j"} == 1)
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Neo4j cluster node is down"
          description: "Neo4j instance {{ $labels.instance }} has been down for more than 5 minutes"

      - alert: Neo4jClusterSizeTooSmall
        expr: neo4j_cluster_core_size < 3
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Neo4j cluster size is too small"
          description: "Cluster has only {{ $value }} core members, minimum is 3"

      # Transaction log
      - alert: Neo4jTxLogGrowth
        expr: rate(neo4j_tx_log_size_in_bytes[5m]) > 10485760
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Neo4j transaction log growing too fast"
          description: "Transaction log growing at {{ $value | humanize }} bytes/second"

      # Memory
      - alert: Neo4jHeapUsageHigh
        expr: neo4j_jvm_heap_used_ratio > 0.85
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Neo4j heap usage is high"
          description: "Heap usage is {{ $value | humanizePercentage }} on {{ $labels.instance }}"

      - alert: Neo4jPageCacheEvictions
        expr: rate(neo4j_page_cache_evictions[5m]) > 1000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Neo4j page cache evictions are high"
          description: "{{ $value }} page cache evictions per second"

      # Query performance
      - alert: Neo4jSlowQueries
        expr: rate(neo4j_query_slow[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Neo4j slow queries detected"
          description: "{{ $value }} slow queries per second"

      - alert: Neo4jQueryQueueHigh
        expr: neo4j_query_queue_length > 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Neo4j query queue is backing up"
          description: "{{ $value }} queries waiting execution"
```

### Grafana Dashboard Panels

```json
{
  "dashboard": {
    "title": "Neo4j Cluster Overview",
    "panels": [
      {
        "title": "Cluster Size",
        "type": "stat",
        "targets": [
          {
            "expr": "neo4j_cluster_core_size",
            "legendFormat": "Core Members"
          }
        ]
      },
      {
        "title": "Transactions per Second",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(neo4j_transactions_committed[1m])",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Heap Memory Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "neo4j_jvm_heap_used_ratio * 100",
            "legendFormat": "{{ instance }} - Heap Used %",
            "unit": "percent"
          }
        ]
      },
      {
        "title": "Page Cache Hit Ratio",
        "type": "graph",
        "targets": [
          {
            "expr": "neo4j_page_cache_hit_ratio * 100",
            "legendFormat": "{{ instance }}",
            "unit": "percent"
          }
        ]
      },
      {
        "title": "Query Execution Time (95th percentile)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(neo4j_query_execution_time_bucket[5m]))",
            "legendFormat": "p95",
            "unit": "ms"
          }
        ]
      }
    ]
  }
}
```

## Part 3: Backup and Recovery

### Backup Strategy

```bash
#!/bin/bash
# Neo4j Online Backup Script

NEO4J_HOME="/var/lib/neo4j"
BACKUP_DIR="/backup/neo4j"
DATE=$(date +%Y%m%d_%H%M%S)
RETENTION_DAYS=7

# Full backup
neo4j-admin database backup neo4j \
    --to-path=$BACKUP_DIR \
    --type=full

# Verify backup integrity
neo4j-admin database verify \
    --backup-path=$BACKUP_DIR

# Clean old backups
find $BACKUP_DIR -type f -mtime +$RETENTION_DAYS -delete

echo "Backup completed: $DATE"
```

```yaml
# Kubernetes CronJob for backups

apiVersion: batch/v1
kind: CronJob
metadata:
  name: neo4j-backup
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
            - name: neo4j-backup
              image: neo4j:5.15-enterprise
              command:
                - /bin/sh
                - -c
                - |
                  neo4j-admin database backup neo4j \
                    --to-path=/backup \
                    --type=full
                  neo4j-admin database verify --backup-path=/backup
                  find /backup -type f -mtime +7 -delete
              env:
                - name: NEO4J_AUTH
                  valueFrom:
                    secretKeyRef:
                      name: neo4j-secret
                      key: password
              volumeMounts:
                - name: backup
                  mountPath: /backup
          volumes:
            - name: backup
              persistentVolumeClaim:
                claimName: neo4j-backup-pvc
          restartPolicy: OnFailure
```

### Recovery Procedures

```bash
# Restore from backup

# 1. Stop Neo4j
neo4j stop

# 2. Restore database
neo4j-admin database restore neo4j \
    --from-path=/backup/neo4j \
    --overwrite-destination=true

# 3. Start Neo4j
neo4j start

# 4. Verify
cypher-shell "RETURN count(*)"
```

```bash
# Disaster Recovery - Rebuild cluster

# 1. Start single node (recovery mode)
NEO4J_dbms_allow__upgrade=true \
NEO4J_dbms_mode=SINGLE \
neo4j start

# 2. Verify data
cypher-shell "MATCH (n) RETURN count(n)"

# 3. Re-enable clustering
# Update neo4j.conf with cluster settings

# 4. Start additional cores
# They will discover and sync automatically
```

## Part 4: Performance Tuning

### Memory Configuration

```yaml
# neo4j.conf - Memory Settings

# Heap sizing (follows JVM guidelines)
# Set both to same value to prevent resizing
dbms.memory.heap.initial_size=4g
dbms.memory.heap.max_size=4g

# Page cache (memory-mapped file cache)
# Should be: Total RAM - Heap - OS overhead (1-2GB)
# Example: 16GB RAM - 4GB Heap - 2GB OS = 10GB Page Cache
dbms.memory.pagecache.size=10g

# GC Settings (in neo4j.conf or JAVA_OPTS)
# Use G1GC for large heaps
wrapper.java.additional=-XX:+UseG1GC
wrapper.java.additional=-XX:MaxGCPauseMillis=200
wrapper.java.additional=-XX:InitiatingHeapOccupancyPercent=75
```

### Query Performance

```cypher
-- Monitoring slow queries
CALL dbms.listQueries()
YIELD query, executionTime, status
WHERE executionTime > 1000  -- > 1 second
RETURN query, executionTime
ORDER BY executionTime DESC;

-- Query statistics
CALL dbms.listQueries()
YIELD query, executions, totalTime
RETURN query, executions, totalTime, totalTime/executions as avgTime
ORDER BY totalTime DESC;

-- Index usage statistics
CALL db.indexes()
YIELD name, type, state, userDescription, indexProvider
RETURN name, state, userDescription;

-- Constraint verification
CALL db.constraints()
YIELD name, type, labels, properties, ownedIndex
RETURN name, type, labels, properties;
```

```cypher
-- Optimize query with EXPLAIN/PROFILE

-- EXPLAIN shows planned execution (doesn't run)
EXPLAIN MATCH (p:Person {name: "Alice"})-[:FRIENDS_OF*2]->(fof)
RETURN fof.name;

-- PROFILE shows actual execution with metrics
PROFILE MATCH (p:Person {name: "Alice"})-[:FRIENDS_OF*2]->(fof)
WHERE fof.age > 25
RETURN fof.name;

-- Key metrics to check:
-- - DB Hits (lower is better)
-- - Rows (cardinality estimates vs actual)
-- - Memory (bytes allocated)
-- - Time (execution time)
```

### Bulk Import

```bash
# neo4j-admin import for initial data load

neo4j-admin import \
    --database=neo4j \
    --nodes=persons=persons_header.csv,persons.csv \
    --relationships=friends=friends_header.csv,friends.csv \
    --skip-bad-relationships=true \
    --skip-duplicate-nodes=true
```

```csv
# persons_header.csv
:id,name:STRING,age:INTEGER,email:STRING
person_id,,,

# persons.csv
1,Alice,30,alice@example.com
2,Bob,25,bob@example.com
3,Carol,35,carol@example.com

# friends_header.csv
:START_ID,:END_ID,since:STRING
person_id,person_id,

# friends.csv
1,2,2020-01-15
2,3,2019-06-20
1,3,2021-03-10
```

```rust
// Rust batch import helper

use neo4rs::{Graph, query};

pub async fn batch_import_users(
    graph: &Graph,
    users: Vec<User>,
    batch_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for chunk in users.chunks(batch_size) {
        // Use UNWIND for batch inserts
        let users_json: Vec<serde_json::Value> = chunk
            .iter()
            .map(|u| {
                json!({
                    "id": u.id,
                    "name": u.name,
                    "email": u.email
                })
            })
            .collect();

        graph.run(
            query(
                "UNWIND $users AS u
                 CREATE (p:Person {
                     id: u.id,
                     name: u.name,
                     email: u.email
                 })"
            )
            .param("users", users_json),
        ).await?;
    }

    Ok(())
}
```

---

*This document is part of the Neodatabase exploration series. See [exploration.md](./exploration.md) for the complete index.*
