---
title: "Production-Grade TigerBeetle"
subtitle: "Deployment, compliance, monitoring, and operations for financial infrastructure"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.tigerbeetle
related: exploration.md, 00-zero-to-ledger-engineer.md
---

# Production-Grade TigerBeetle

## Overview

This document covers production deployment of TigerBeetle - compliance requirements, cluster configuration, monitoring strategies, backup/recovery, and operational procedures for financial infrastructure.

## Part 1: Compliance and Regulatory Requirements

### Financial Compliance Framework

```
TigerBeetle for Financial Services:

┌───────────────────────────────────────────────────────────┐
│ SOC 2 Type II Compliance                                   │
│                                                          │
│ Requirements:                                            │
│ - Audit logging (all transactions)                       │
│ - Access controls (role-based)                           │
│ - Encryption (at-rest, in-transit)                       │
│ - Backup and recovery procedures                         │
│ - Change management                                      │
│                                                          │
│ TigerBeetle provides:                                    │
│ ✓ Immutable audit trail (all transfers logged)           │
│ ✓ Replicated storage (durability)                        │
│ ✓ TLS support (encryption in-transit)                    │
│ ✓ Checkpoint/WAL (recovery)                              │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ PCI DSS Compliance (Payment Card Industry)                 │
│                                                          │
│ Requirements for payment processors:                     │
│ - Cardholder data protection                             │
│ - Access control and authentication                      │
│ - Network security                                       │
│ - Monitoring and logging                                 │
│                                                          │
│ TigerBeetle deployment considerations:                   │
│ - Store only tokenized card data                         │
│ - Network segmentation (VPC, security groups)            │
│ - Audit log retention (minimum 1 year)                   │
│ - Regular security assessments                           │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ GDPR Compliance (EU Data Protection)                       │
│                                                          │
│ Requirements:                                            │
│ - Right to erasure ("right to be forgotten")             │
│ - Data portability                                       │
│ - Purpose limitation                                     │
│                                                          │
│ TigerBeetle considerations:                              │
│ - Account data should not include PII                    │
│ - Use external user_data references                      │
│ - Implement data retention policies                      │
│ - Audit trail is immutable (design feature)              │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Basel III / Dodd-Frank (Banking Regulations)               │
│                                                          │
│ Requirements for banks:                                  │
│ - Capital adequacy                                       │
│ - Risk management                                        │
│ - Stress testing                                         │
│ - Transaction reporting                                  │
│                                                          │
│ TigerBeetle supports:                                    │
│ ✓ Real-time balance tracking                             │
│ ✓ Complete transaction history                           │
│ ✓ Multi-ledger for regulatory reporting                  │
│ ✓ High availability (cluster deployment)                 │
└───────────────────────────────────────────────────────────┘
```

### Audit Trail Requirements

```
Audit Trail Implementation:

┌───────────────────────────────────────────────────────────┐
│ Immutable Transaction Log                                  │
│                                                          │
│ Every transaction is:                                    │
│ 1. Assigned unique ID                                    │
│ 2. Timestamped (nanosecond precision)                    │
│ 3. Written to WAL before execution                       │
│ 4. Replicated to quorum                                  │
│ 5. Never modified or deleted                             │
│                                                          │
│ Audit query examples:                                    │
│ - "Show all transfers for account X in date range"       │
│ - "Reconstruct balance at any point in time"             │
│ - "Trace origin of funds through transfer chain"         │
└───────────────────────────────────────────────────────────┘

Audit Log Export:
```rust
/// Export audit log for compliance
struct AuditExport {
    /// Start timestamp (inclusive)
    start_timestamp: u64,

    /// End timestamp (inclusive)
    end_timestamp: u64,

    /// Ledger filter (optional)
    ledger_filter: Option<u32>,

    /// Account filter (optional)
    account_filter: Option<u128>,
}

struct AuditEntry {
    /// Transfer ID
    id: u128,

    /// Timestamp (nanoseconds since epoch)
    timestamp: u64,

    /// Debit account
    debit_account: u128,

    /// Credit account
    credit_account: u128,

    /// Amount
    amount: u64,

    /// Ledger
    ledger: u32,

    /// Code (transaction type)
    code: u16,

    /// User data (external reference)
    user_data: u128,
}

impl TigerBeetleClient {
    /// Export audit log in CSV format
    fn export_audit_log(&self, export: AuditExport) -> Vec<AuditEntry> {
        // Query transfers in date range
        let transfers = self.lookup_transfers_in_range(
            export.start_timestamp,
            export.end_timestamp,
        );

        // Filter by ledger and account if specified
        let mut entries: Vec<AuditEntry> = transfers
            .into_iter()
            .filter(|t| {
                export.ledger_filter.map_or(true, |l| t.ledger == l)
            })
            .filter(|t| {
                export.account_filter.map_or(true, |a| {
                    t.debit_account_id == a || t.credit_account_id == a
                })
            })
            .map(|t| AuditEntry {
                id: t.id,
                timestamp: t.timestamp,
                debit_account: t.debit_account_id,
                credit_account: t.credit_account_id,
                amount: t.amount,
                ledger: t.ledger,
                code: t.code,
                user_data: t.user_data,
            })
            .collect();

        // Sort by timestamp
        entries.sort_by_key(|e| e.timestamp);

        entries
    }

    /// Export to CSV for compliance reporting
    fn export_to_csv(&self, entries: Vec<AuditEntry>, path: &str) {
        let mut writer = csv::Writer::from_path(path).unwrap();

        // Write header
        writer.write_record(&[
            "transfer_id",
            "timestamp",
            "debit_account",
            "credit_account",
            "amount",
            "ledger",
            "code",
            "user_data",
        ]).unwrap();

        // Write entries
        for entry in entries {
            writer.write_record(&[
                entry.id.to_string(),
                entry.timestamp.to_string(),
                entry.debit_account.to_string(),
                entry.credit_account.to_string(),
                entry.amount.to_string(),
                entry.ledger.to_string(),
                entry.code.to_string(),
                entry.user_data.to_string(),
            ]).unwrap();
        }

        writer.flush().unwrap();
    }
}
```

## Part 2: Cluster Deployment

### Production Cluster Architecture

```
3-Replica Cluster (Single Region):

┌───────────────────────────────────────────────────────────┐
│                     Single Region                          │
│                                                          │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐    │
│  │  Replica 0  │   │  Replica 1  │   │  Replica 2  │    │
│  │   (AZ-a)    │   │   (AZ-b)    │   │   (AZ-c)    │    │
│  │   Leader    │◄──│  Follower   │◄──│  Follower   │    │
│  └──────┬──────┘   └──────┬──────┘   └──────┬──────┘    │
│         │                 │                 │            │
│         └─────────────────┼─────────────────┘            │
│                           │                              │
│                  ┌────────▼────────┐                     │
│                  │  Load Balancer  │                     │
│                  │   (NLB/ALB)     │                     │
│                  └────────┬────────┘                     │
│                           │                              │
│                  ┌────────▼────────┐                     │
│                  │   Application   │                     │
│                  └─────────────────┘                     │
│                                                          │
│ Availability Zones: 3 (a, b, c)                          │
│ Failure tolerance: 1 AZ failure                          │
│ Read latency: < 1ms (same region)                        │
│ Write latency: 2-5ms (quorum replication)                │
└───────────────────────────────────────────────────────────┘

5-Replica Cluster (Multi-Region):

┌───────────────────────────────────────────────────────────┐
│                     Multi-Region                           │
│                                                          │
│  US-East (3 replicas)          US-West (2 replicas)       │
│  ┌────────┐ ┌────────┐ ┌────────┐    ┌────────┐ ┌────────┐│
│  │ Rep 0  │ │ Rep 1  │ │ Rep 2  │    │ Rep 3  │ │ Rep 4  ││
│  │ Leader │ │Follow  │ │Follow  │    │Follow  │ │Follow  ││
│  └───┬────┘ └───┬────┘ └───┬────┘    └───┬────┘ └───┬────┘│
│      │          │          │             │          │     │
│      └──────────┼──────────┘             └──────────┘     │
│                 │                          │              │
│          ┌──────▼────────┐          ┌──────▼────────┐    │
│          │  US-East NLB  │          │  US-West NLB  │    │
│          └───────┬───────┘          └───────┬───────┘    │
│                  │                          │             │
│           ┌──────┴──────────────────────────┘             │
│           │                                               │
│    ┌──────▼────────┐                                      │
│    │ Global DNS    │ (Route53, GeoDNS)                   │
│    │ Route to nearest│                                     │
│    └───────┬───────┘                                      │
│            │                                               │
│    ┌───────▼────────┐                                     │
│    │  Application   │                                     │
│    └────────────────┘                                     │
│                                                          │
│ Regions: 2 (US-East, US-West)                            │
│ Failure tolerance: 2 replica failures OR 1 region        │
│ Read latency: < 1ms (local), 50-100ms (cross-region)     │
│ Write latency: 100-200ms (cross-region quorum)           │
└───────────────────────────────────────────────────────────┘
```

### Docker Deployment

```dockerfile
# Dockerfile for TigerBeetle
FROM scratch

# Copy TigerBeetle binary
COPY tigerbeetle /tigerbeetle

# Copy configuration
COPY tigerbeetle.conf /etc/tigerbeetle/tigerbeetle.conf

# Data directory
VOLUME /data

# Ports
# 3000: Cluster communication
# 3001: Client API
EXPOSE 3000 3001

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/tigerbeetle", "format", "--check", "/data"]

# Run
ENTRYPOINT ["/tigerbeetle"]
CMD ["server", "--config", "/etc/tigerbeetle/tigerbeetle.conf"]
```

```yaml
# docker-compose.yml for local testing
version: '3.8'

services:
  tigerbeetle-0:
    build: .
    container_name: tigerbeetle-0
    volumes:
      - ./data/replica-0:/data
      - ./config/replica-0.conf:/etc/tigerbeetle/tigerbeetle.conf
    ports:
      - "3000:3000"
      - "3001:3001"
    networks:
      - tigerbeetle
    healthcheck:
      test: ["CMD", "/tigerbeetle", "format", "--check", "/data"]
      interval: 10s
      timeout: 5s
      retries: 3

  tigerbeetle-1:
    build: .
    container_name: tigerbeetle-1
    volumes:
      - ./data/replica-1:/data
      - ./config/replica-1.conf:/etc/tigerbeetle/tigerbeetle.conf
    ports:
      - "3010:3000"
      - "3011:3001"
    networks:
      - tigerbeetle
    depends_on:
      tigerbeetle-0:
        condition: service_healthy

  tigerbeetle-2:
    build: .
    container_name: tigerbeetle-2
    volumes:
      - ./data/replica-2:/data
      - ./config/replica-2.conf:/etc/tigerbeetle/tigerbeetle.conf
    ports:
      - "3020:3000"
      - "3021:3001"
    networks:
      - tigerbeetle
    depends_on:
      tigerbeetle-0:
        condition: service_healthy

networks:
  tigerbeetle:
    driver: bridge
```

### Kubernetes Deployment

```yaml
# tigerbeetle-statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: tigerbeetle
  namespace: finance
spec:
  serviceName: tigerbeetle
  replicas: 3
  selector:
    matchLabels:
      app: tigerbeetle
  template:
    metadata:
      labels:
        app: tigerbeetle
    spec:
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            - labelSelector:
                matchExpressions:
                  - key: app
                    operator: In
                    values:
                      - tigerbeetle
              topologyKey: topology.kubernetes.io/zone
      containers:
        - name: tigerbeetle
          image: tigerbeetle/tigerbeetle:latest
          args:
            - server
            - --config
            - /etc/tigerbeetle/tigerbeetle.conf
          ports:
            - containerPort: 3000
              name: cluster
            - containerPort: 3001
              name: api
          resources:
            requests:
              cpu: "2"
              memory: "4Gi"
            limits:
              cpu: "4"
              memory: "8Gi"
          volumeMounts:
            - name: data
              mountPath: /data
            - name: config
              mountPath: /etc/tigerbeetle
          livenessProbe:
            tcpSocket:
              port: 3001
            initialDelaySeconds: 30
            periodSeconds: 10
            timeoutSeconds: 5
          readinessProbe:
            exec:
              command:
                - /tigerbeetle
                - format
                - --check
                - /data
            initialDelaySeconds: 10
            periodSeconds: 5
            timeoutSeconds: 5
  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        accessModes: ["ReadWriteOnce"]
        storageClassName: gp3
        resources:
          requests:
            storage: 100Gi
        # IOPS provisioned for WAL performance
        annotations:
          volume.beta.kubernetes.io/storage-provisioner: ebs.csi.aws.com
      volumeMode: Filesystem
  ---
apiVersion: v1
kind: Service
metadata:
  name: tigerbeetle
  namespace: finance
spec:
  clusterIP: None
  ports:
    - port: 3001
      targetPort: 3001
      name: api
  selector:
    app: tigerbeetle
  ---
apiVersion: v1
kind: Service
metadata:
  name: tigerbeetle-lb
  namespace: finance
spec:
  type: LoadBalancer
  ports:
    - port: 3001
      targetPort: 3001
      name: api
  selector:
    app: tigerbeetle
```

### Configuration Files

```bash
# TigerBeetle configuration (replica-0.conf)

# Replica identity
replica = 0
replica_count = 3

# Cluster configuration
cluster = 0x1234567890abcdef1234567890abcdef  # Unique cluster ID

# Network addresses
address = 0.0.0.0:3001
peer_address = 0.0.0.0:3000

# Peer addresses (all replicas)
peers = [
    "10.0.1.10:3000",  # Replica 0
    "10.0.1.11:3000",  # Replica 1
    "10.0.1.12:3000",  # Replica 2
]

# Storage configuration
data_dir = /data

# Performance tuning
write_mode = direct  # Direct I/O (bypass page cache)
read_mode = direct

# Checkpoint configuration
checkpoint_interval = 60  # seconds
checkpoint_count = 3      # Keep 3 checkpoints

# Log configuration
log_level = info
log_dir = /var/log/tigerbeetle

# Security
tls_cert = /etc/tls/tigerbeetle.crt
tls_key = /etc/tls/tigerbeetle.key
tls_ca = /etc/tls/ca.crt

# Admin authentication
admin_token = super_secure_admin_token_here
```

## Part 3: Monitoring and Alerting

### Key Metrics

```
TigerBeetle Metrics (Prometheus format):

┌───────────────────────────────────────────────────────────┐
│ Replication Metrics                                        │
│                                                          │
│ tigerbeetle_replica_role{replica="0"}                    │
│   Gauge: 1 = Leader, 0 = Follower                        │
│                                                          │
│ tigerbeetle_replica_view{replica="0"}                    │
│   Gauge: Current view number                             │
│                                                          │
│ tigerbeetle_replication_lag_bytes{replica="1"}           │
│   Gauge: Bytes behind leader                             │
│                                                          │
│ tigerbeetle_replication_connected{replica="1"}           │
│   Gauge: 1 = Connected to leader, 0 = Disconnected       │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Transaction Metrics                                        │
│                                                          │
│ tigerbeetle_transactions_total                           │
│   Counter: Total transactions processed                  │
│                                                          │
│ tigerbeetle_transactions_success_total                   │
│   Counter: Successful transactions                       │
│                                                          │
│ tigerbeetle_transactions_error_total{error_type="..."}   │
│   Counter: Failed transactions by error type             │
│                                                          │
│ tigerbeetle_transaction_latency_seconds                  │
│   Histogram: Transaction processing latency              │
│                                                          │
│ tigerbeetle_transaction_queue_length                     │
│   Gauge: Pending transactions in queue                   │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Storage Metrics                                            │
│                                                          │
│ tigerbeetle_storage_used_bytes                           │
│   Gauge: Storage used                                    │
│                                                          │
│ tigerbeetle_storage_total_bytes                          │
│   Gauge: Total storage capacity                          │
│                                                          │
│ tigerbeetle_wal_entries_total                            │
│   Counter: Total WAL entries written                     │
│                                                          │
│ tigerbeetle_wal_bytes_written_total                      │
│   Counter: Total WAL bytes written                       │
│                                                          │
│ tigerbeetle_checkpoint_duration_seconds                  │
│   Histogram: Checkpoint duration                         │
│                                                          │
│ tigerbeetle_checkpoint_age_seconds                       │
│   Gauge: Time since last checkpoint                      │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Account Metrics                                            │
│                                                          │
│ tigerbeetle_accounts_total                               │
│   Gauge: Total number of accounts                        │
│                                                          │
│ tigerbeetle_accounts_created_total                       │
│   Counter: Accounts created                              │
│                                                          │
│ tigerbeetle_account_balance_sum{ledger="700"}            │
│   Gauge: Sum of all balances in ledger                   │
│                                                          │
│ tigerbeetle_transfers_total                              │
│   Counter: Total transfers created                       │
└───────────────────────────────────────────────────────────┘
```

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'tigerbeetle'
    static_configs:
      - targets:
          - 'tigerbeetle-0.tigerbeetle.finance.svc:9090'
          - 'tigerbeetle-1.tigerbeetle.finance.svc:9090'
          - 'tigerbeetle-2.tigerbeetle.finance.svc:9090'
    metrics_path: /metrics
    scrape_interval: 5s

  - job_name: 'tigerbeetle-node-exporter'
    static_configs:
      - targets:
          - 'tigerbeetle-0.tigerbeetle.finance.svc:9100'
          - 'tigerbeetle-1.tigerbeetle.finance.svc:9100'
          - 'tigerbeetle-2.tigerbeetle.finance.svc:9100'

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

rule_files:
  - 'tigerbeetle_alerts.yml'
```

### Alert Rules

```yaml
# tigerbeetle_alerts.yml
groups:
  - name: tigerbeetle
    rules:
      # Leader election alerts
      - alert: TigerBeetleNoLeader
        expr: sum(tigerbeetle_replica_role) == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "TigerBeetle cluster has no leader"
          description: "All replicas are followers - cluster cannot process writes"

      - alert: TigerBeetleFrequentElections
        expr: rate(tigerbeetle_replica_view[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "TigerBeetle frequent leader elections"
          description: "View changing {{ $value }} times per second"

      # Replication lag alerts
      - alert: TigerBeetleReplicationLag
        expr: tigerbeetle_replication_lag_bytes > 10000000
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "TigerBeetle replication lag high"
          description: "Replica {{ $labels.replica }} is {{ $value | humanize }} bytes behind"

      - alert: TigerBeetleReplicaDisconnected
        expr: tigerbeetle_replication_connected == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "TigerBeetle replica disconnected"
          description: "Replica {{ $labels.replica }} is not connected to leader"

      # Storage alerts
      - alert: TigerBeetleStorageHigh
        expr: tigerbeetle_storage_used_bytes / tigerbeetle_storage_total_bytes > 0.85
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "TigerBeetle storage usage above 85%"
          description: "Storage usage is {{ $value | humanizePercentage }}"

      - alert: TigerBeetleStorageCritical
        expr: tigerbeetle_storage_used_bytes / tigerbeetle_storage_total_bytes > 0.95
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "TigerBeetle storage usage above 95%"
          description: "Immediate action required - cluster may become read-only"

      - alert: TigerBeetleCheckpointOld
        expr: tigerbeetle_checkpoint_age_seconds > 600
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "TigerBeetle checkpoint is stale"
          description: "Last checkpoint was {{ $value | humanizeDuration }} ago"

      # Transaction error alerts
      - alert: TigerBeetleTransactionErrors
        expr: rate(tigerbeetle_transactions_error_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High TigerBeetle transaction error rate"
          description: "{{ $value }} errors per second"

      - alert: TigerBeetleInsufficientFunds
        expr: rate(tigerbeetle_transactions_error_total{error_type="insufficient_funds"}[5m]) > 100
        for: 5m
        labels:
          severity: info
        annotations:
          summary: "High rate of insufficient funds errors"
          description: "May indicate application logic issue or fraud attempt"

      # Latency alerts
      - alert: TigerBeetleHighLatency
        expr: histogram_quantile(0.99, rate(tigerbeetle_transaction_latency_seconds_bucket[5m])) > 0.01
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "TigerBeetle P99 latency above 10ms"
          description: "P99 latency is {{ $value | humanizeDuration }}"
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "TigerBeetle Cluster Overview",
    "panels": [
      {
        "title": "Cluster Health",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(tigerbeetle_replica_role)",
            "legendFormat": "Leaders"
          },
          {
            "expr": "count(tigerbeetle_replica_role) - sum(tigerbeetle_replica_role)",
            "legendFormat": "Followers"
          }
        ]
      },
      {
        "title": "Transactions per Second",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(tigerbeetle_transactions_total[1m])",
            "legendFormat": "TPS"
          },
          {
            "expr": "rate(tigerbeetle_transactions_success_total[1m])",
            "legendFormat": "Success TPS"
          }
        ]
      },
      {
        "title": "Transaction Latency",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(tigerbeetle_transaction_latency_seconds_bucket[5m]))",
            "legendFormat": "P50"
          },
          {
            "expr": "histogram_quantile(0.95, rate(tigerbeetle_transaction_latency_seconds_bucket[5m]))",
            "legendFormat": "P95"
          },
          {
            "expr": "histogram_quantile(0.99, rate(tigerbeetle_transaction_latency_seconds_bucket[5m]))",
            "legendFormat": "P99"
          }
        ]
      },
      {
        "title": "Replication Lag",
        "type": "graph",
        "targets": [
          {
            "expr": "tigerbeetle_replication_lag_bytes",
            "legendFormat": "Replica {{ replica }}"
          }
        ]
      },
      {
        "title": "Storage Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "tigerbeetle_storage_used_bytes / tigerbeetle_storage_total_bytes * 100",
            "legendFormat": "Usage %"
          }
        ]
      },
      {
        "title": "Account Balances by Ledger",
        "type": "graph",
        "targets": [
          {
            "expr": "tigerbeetle_account_balance_sum",
            "legendFormat": "Ledger {{ ledger }}"
          }
        ]
      }
    ]
  }
}
```

## Part 4: Backup and Recovery

### Backup Strategy

```
Backup Architecture:

┌───────────────────────────────────────────────────────────┐
│ Continuous Backup                                          │
│                                                          │
│ 1. WAL Archival                                          │
│    - Copy WAL segments to S3/GCS after rotation          │
│    - Retention: 30 days minimum                          │
│    - Encryption: SSE-S3 or customer-managed keys         │
│                                                          │
│ 2. Periodic Checkpoints                                  │
│    - Export checkpoint to cold storage monthly           │
│    - Retention: 7 years (regulatory requirement)         │
│                                                          │
│ 3. Point-in-Time Recovery                                │
│    - Restore from checkpoint + replay WAL                │
│    - RPO: Near-zero (continuous WAL archival)            │
│    - RTO: < 1 hour for typical database                  │
└───────────────────────────────────────────────────────────┘

Backup Schedule:
┌───────────────────────────────────────────────────────────┐
│ Backup Type      │ Frequency │ Retention │ Storage       │
├───────────────────────────────────────────────────────────┤
│ WAL Archive      │ Continuous│ 30 days   │ S3/GCS        │
│ Checkpoint       │ Daily     │ 7 days    │ S3/GCS        │
│ Full Backup      │ Weekly    │ 4 weeks   │ S3/GCS + GL   │
│ Monthly Snapshot │ Monthly   │ 7 years   │ Glacier       │
└───────────────────────────────────────────────────────────┘
```

### Backup Script

```bash
#!/bin/bash
# backup_tigerbeetle.sh

set -e

# Configuration
CLUSTER_ID="0x1234567890abcdef1234567890abcdef"
S3_BUCKET="tigerbeetle-backups"
BACKUP_DIR="/tmp/tigerbeetle-backup"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
RETENTION_DAYS=30

# Create backup directory
mkdir -p $BACKUP_DIR

echo "Starting TigerBeetle backup at $(date)"

# Trigger checkpoint on all replicas
for replica in 0 1 2; do
    echo "Triggering checkpoint on replica $replica..."
    tigerbeetle format --check /data/replica-$replica || true
done

# Copy data files
echo "Copying data files..."
for replica in 0 1 2; do
    DATA_FILE="/data/replica-$replica/0_0_0.tigerbeetle"
    if [ -f "$DATA_FILE" ]; then
        cp "$DATA_FILE" "$BACKUP_DIR/replica-$replica-$TIMESTAMP.tigerbeetle"
    fi
done

# Compress backup
echo "Compressing backup..."
tar -czf "$BACKUP_DIR/backup-$TIMESTAMP.tar.gz" -C "$BACKUP_DIR" .

# Upload to S3
echo "Uploading to S3..."
aws s3 cp "$BACKUP_DIR/backup-$TIMESTAMP.tar.gz" \
    "s3://$S3_BUCKET/cluster-$CLUSTER_ID/daily/backup-$TIMESTAMP.tar.gz" \
    --storage-class STANDARD_IA

# Upload WAL segments (if using WAL archival)
echo "Archiving WAL segments..."
for replica in 0 1 2; do
    WAL_DIR="/data/replica-$replica/wal"
    if [ -d "$WAL_DIR" ]; then
        aws s3 sync "$WAL_DIR" \
            "s3://$S3_BUCKET/cluster-$CLUSTER_ID/wal/replica-$replica/" \
            --storage-class STANDARD_IA
    fi
done

# Cleanup old backups
echo "Cleaning up backups older than $RETENTION_DAYS days..."
aws s3 ls "s3://$S3_BUCKET/cluster-$CLUSTER_ID/daily/" | \
    awk -v threshold="$RETENTION_DAYS" '
    {
        # Parse date from filename
        split($2, parts, "-")
        # Simple age check (production should use proper date parsing)
        print $4
    }' | while read file; do
    # Check file age and delete if old
    file_date=$(echo "$file" | grep -oP '\d{8}-\d{6}' | head -1)
    if [ -n "$file_date" ]; then
        file_ts=$(date -d "${file_date:0:4}-${file_date:4:2}-${file_date:6:2}" +%s 2>/dev/null || echo 0)
        now_ts=$(date +%s)
        age_days=$(( (now_ts - file_ts) / 86400 ))
        if [ "$age_days" -gt "$RETENTION_DAYS" ]; then
            echo "Deleting old backup: $file"
            aws s3 rm "s3://$S3_BUCKET/cluster-$CLUSTER_ID/daily/$file"
        fi
    fi
done

# Cleanup local
rm -rf $BACKUP_DIR

echo "Backup completed at $(date)"
```

### Recovery Procedure

```bash
#!/bin/bash
# recover_tigerbeetle.sh

set -e

# Configuration
CLUSTER_ID="0x1234567890abcdef1234567890abcdef"
S3_BUCKET="tigerbeetle-backups"
BACKUP_TO_RESTORE="backup-20260328-020000.tar.gz"
DATA_DIR="/data/replica-0"

echo "Starting TigerBeetle recovery at $(date)"

# Stop TigerBeetle service
echo "Stopping TigerBeetle..."
systemctl stop tigerbeetle || true

# Clear existing data
echo "Clearing existing data..."
rm -rf $DATA_DIR/*
mkdir -p $DATA_DIR

# Download backup from S3
echo "Downloading backup from S3..."
aws s3 cp "s3://$S3_BUCKET/cluster-$CLUSTER_ID/daily/$BACKUP_TO_RESTORE" \
    /tmp/backup.tar.gz

# Extract backup
echo "Extracting backup..."
tar -xzf /tmp/backup.tar.gz -C $DATA_DIR

# Download and replay WAL (for point-in-time recovery)
echo "Downloading WAL segments..."
aws s3 sync "s3://$S3_BUCKET/cluster-$CLUSTER_ID/wal/replica-0/" \
    "$DATA_DIR/wal"

# Replay WAL (TigerBeetle does this automatically on startup)
echo "Replaying WAL..."
# TigerBeetle automatically replays WAL on startup

# Start TigerBeetle
echo "Starting TigerBeetle..."
systemctl start tigerbeetle

# Verify recovery
echo "Verifying recovery..."
sleep 5
tigerbeetle format --check $DATA_DIR

echo "Recovery completed at $(date)"

# Verify cluster health
echo "Verifying cluster health..."
# Add health check commands here
```

## Part 5: Security Hardening

### Network Security

```
Network Architecture:

┌───────────────────────────────────────────────────────────┐
│ VPC Configuration                                          │
│                                                          │
│ ┌─────────────────────────────────────────────────────┐  │
│ │ Private Subnet (TigerBeetle)                        │  │
│ │                                                     │  │
│ │  ┌───────────┐ ┌───────────┐ ┌───────────┐        │  │
│ │  │ Replica 0 │ │ Replica 1 │ │ Replica 2 │        │  │
│ │  │ 10.0.1.10 │ │ 10.0.1.11 │ │ 10.0.1.12 │        │  │
│ │  └───────────┘ └───────────┘ └───────────┘        │  │
│ │                                                     │  │
│ │ Security Group: tigerbeetle-internal                │  │
│ │ - Allow 3000 from tigerbeetle-internal (peers)      │  │
│ │ - Allow 3001 from tigerbeetle-app (API)             │  │
│ │ - Deny all inbound from internet                    │  │
│ └─────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌─────────────────────────────────────────────────────┐  │
│ │ Private Subnet (Application)                        │  │
│ │                                                     │  │
│ │  ┌───────────────────────────┐                     │  │
│ │  │ Application Servers       │                     │  │
│ │  └───────────────────────────┘                     │  │
│ │                                                     │  │
│ │ Security Group: tigerbeetle-app                     │  │
│ │ - Allow outbound to tigerbeetle-internal:3001       │  │
│ └─────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌─────────────────────────────────────────────────────┐  │
│ │ Public Subnet (Bastion/NAT)                         │  │
│ │                                                     │  │
│ │  - Bastion host for SSH access                      │  │
│ │  - NAT Gateway for outbound traffic                 │  │
│ └─────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────┘

Security Group Rules:
- tigerbeetle-internal: Allow 3000, 3001 from VPC CIDR
- tigerbeetle-app: Allow outbound 3001 to tigerbeetle-internal
- No public IP addresses on TigerBeetle instances
```

### TLS Configuration

```bash
# Generate TLS certificates

# 1. Create CA
openssl genrsa -out ca.key 4096
openssl req -new -x509 -days 3650 -key ca.key -out ca.crt \
    -subj "/CN=TigerBeetle CA/O=Finance/C=US"

# 2. Generate server certificate
openssl genrsa -out tigerbeetle.key 2048
openssl req -new -key tigerbeetle.key -out tigerbeetle.csr \
    -subj "/CN=tigerbeetle.finance.svc/O=Finance/C=US"

# 3. Sign server certificate
openssl x509 -req -days 365 -in tigerbeetle.csr -CA ca.crt -CAkey ca.key \
    -CAcreateserial -out tigerbeetle.crt \
    -extfile <(echo "subjectAltName=DNS:tigerbeetle,DNS:tigerbeetle.finance.svc")

# 4. Distribute certificates
# - ca.crt: All clients and replicas
# - tigerbeetle.crt + tigerbeetle.key: Each replica

# 5. Configure TigerBeetle with TLS
# See tigerbeetle.conf above for TLS settings
```

### Authentication

```rust
/// Client authentication middleware
struct AuthMiddleware {
    /// API tokens database
    tokens: HashMap<String, ApiToken>,
}

struct ApiToken {
    /// Token ID
    id: String,

    /// Associated account/user
    account_id: u128,

    /// Permissions
    permissions: Vec<Permission>,

    /// Expiration timestamp
    expires_at: u64,
}

enum Permission {
    ReadAccounts,
    CreateAccounts,
    ReadTransfers,
    CreateTransfers,
    Admin,
}

impl AuthMiddleware {
    fn validate_token(&self, token: &str) -> Result<&ApiToken, AuthError> {
        let api_token = self.tokens.get(token)
            .ok_or(AuthError::InvalidToken)?;

        if api_token.expires_at < current_timestamp_ns() {
            return Err(AuthError::TokenExpired);
        }

        Ok(api_token)
    }

    fn check_permission(&self, token: &ApiToken, required: Permission) -> Result<(), AuthError> {
        if !token.permissions.contains(&required) {
            return Err(AuthError::InsufficientPermissions);
        }
        Ok(())
    }
}
```

---

*This document is part of the TigerBeetle exploration series. See [exploration.md](./exploration.md) for the complete index.*
