# Production-Grade Time-Series Database: Operational Guide

**Based on TimescaleDB and pgvectorscale production deployments**

---

## Table of Contents

1. [Deployment Architecture](#deployment-architecture)
2. [High Availability](#high-availability)
3. [Backup and Recovery](#backup-and-recovery)
4. [Monitoring and Observability](#monitoring-and-observability)
5. [Performance Tuning](#performance-tuning)
6. [Security](#security)
7. [Scaling Strategies](#scaling-strategies)
8. [Troubleshooting](#troubleshooting)

---

## Deployment Architecture

### Single Node Deployment

```
┌────────────────────────────────────────────────────────────┐
│                    SINGLE NODE                               │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  Application                         │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              TimescaleDB Instance                    │   │
│  │  ┌─────────────┬─────────────┬─────────────────┐    │   │
│  │  │  Hypertable │  Continuous │  Compression    │    │   │
│  │  │  Manager    │  Aggregates │  Engine         │    │   │
│  │  └─────────────┴─────────────┴─────────────────┘    │   │
│  │  ┌─────────────┬─────────────┬─────────────────┐    │   │
│  │  │  pgvectorscale │  Toolkit  │  Background     │    │   │
│  │  │  (Vector)    │  Functions │  Worker         │    │   │
│  │  └─────────────┴─────────────┴─────────────────┘    │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  Storage (SSD)                       │   │
│  │  - Data directory                                    │   │
│  │  - WAL directory (separate disk recommended)         │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Multi-Node with Replication

```
┌────────────────────────────────────────────────────────────┐
│                    MULTI-NODE CLUSTER                        │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────┐      ┌──────────────────┐            │
│  │  Primary Node    │─────>│  Replica Node 1  │            │
│  │  (Read/Write)    │      │  (Read Only)     │            │
│  │                  │      │                  │            │
│  │  ┌────────────┐  │      │  ┌────────────┐  │            │
│  │  │ Hypertable │  │      │  │ Hypertable │  │            │
│  │  │ (All data) │  │      │  │ (Mirrored) │  │            │
│  │  └────────────┘  │      │  └────────────┘  │            │
│  └──────────────────┘      └──────────────────┘            │
│         │                                                   │
│         │ Streaming Replication                             │
│         ▼                                                   │
│  ┌──────────────────┐                                      │
│  │  Replica Node 2  │                                      │
│  │  (Read Only)     │                                      │
│  │                  │                                      │
│  │  ┌────────────┐  │                                      │
│  │  │ Hypertable │  │                                      │
│  │  │ (Mirrored) │  │                                      │
│  │  └────────────┘  │                                      │
│  └──────────────────┘                                      │
│                                                             │
│  Load Balancer (pgpool-II or similar)                      │
│  - Routes writes to primary                                │
│  - Distributes reads across replicas                       │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Distributed Hypertable Deployment

```
┌────────────────────────────────────────────────────────────┐
│                 DISTRIBUTED HYPERTABLE                       │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────┐      ┌──────────────────┐            │
│  │  Access Node     │      │  Access Node     │            │
│  │  (Coordinator)   │      │  (Coordinator)   │            │
│  │                  │      │                  │            │
│  │  Hypertable      │      │  Hypertable      │            │
│  │  (Metadata)      │      │  (Metadata)      │            │
│  └────────┬─────────┘      └────────┬─────────┘            │
│           │                         │                       │
│           └───────────┬─────────────┘                       │
│                       │                                     │
│         ┌─────────────┼─────────────┐                       │
│         │             │             │                       │
│         ▼             ▼             ▼                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │
│  │ Data Node│  │ Data Node│  │ Data Node│                 │
│  │    1     │  │    2     │  │    3     │                 │
│  │          │  │          │  │          │                 │
│  │ Chunks   │  │ Chunks   │  │ Chunks   │                 │
│  └──────────┘  └──────────┘  └──────────┘                 │
│                                                             │
│  - Chunks distributed across data nodes                    │
│  - Query routing handled by access nodes                   │
│  - Horizontal write scalability                            │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

---

## High Availability

### Streaming Replication Setup

```sql
-- Primary node configuration (postgresql.conf)
wal_level = replica
max_wal_senders = 10
max_replication_slots = 10
synchronous_standby_names = 'replica1,replica2'
archive_mode = on
archive_command = 'wal-g wal-push %p'

-- Create replication slot
SELECT pg_create_physical_replication_slot('replica1');

-- Replica node configuration (postgresql.conf)
hot_standby = on
primary_conninfo = 'host=primary port=5432 user=replicator password=secret'
primary_slot_name = 'replica1'
hot_standby_feedback = on
```

### Automatic Failover with Patroni

```yaml
# patroni.yml
scope: timescale-cluster
namespace: /db/
name: timescale-1

restapi:
  listen: 0.0.0.0:8008
  connect_address: timescale-1:8008

etcd:
  hosts: etcd-1:2379,etcd-2:2379,etcd-3:2379

bootstrap:
  dcs:
    ttl: 30
    loop_wait: 10
    retry_timeout: 10
    maximum_lag_on_failover: 1048576
    postgresql:
      use_pg_rewind: true
      use_slots: true
      parameters:
        wal_level: replica
        hot_standby: "on"

postgresql:
  listen: 0.0.0.0:5432
  connect_address: timescale-1:5432
  data_dir: /var/lib/postgresql/data
  pg_hba:
    - host replication replicator 0.0.0.0/0 md5
    - host all all 0.0.0.0/0 md5
  parameters:
    shared_buffers: 4GB
    work_mem: 64MB
    maintenance_work_mem: 512MB
```

### Connection Pooling with PgBouncer

```ini
; pgbouncer.ini
[databases]
timescale = host=localhost port=5432 dbname=timescale

[pgbouncer]
listen_port = 6432
listen_addr = 0.0.0.0
auth_type = md5
auth_file = /etc/pgbouncer/userlist.txt
pool_mode = transaction
max_client_conn = 1000
default_pool_size = 25
reserve_pool_size = 5
reserve_pool_timeout = 5
server_lifetime = 3600
server_idle_timeout = 600
```

---

## Backup and Recovery

### WAL-G Configuration

```bash
# /etc/wal-g/wal-g.json
{
  "WALG_S3_PREFIX": "s3://my-backup-bucket/timescale",
  "WALG_S3_REGION": "us-east-1",
  "AWS_ACCESS_KEY_ID": "...",
  "AWS_SECRET_ACCESS_KEY": "...",
  "WALG_COMPRESSION_METHOD": "lz4",
  "WALG_DELTA_MAX_STEPS": 6,
  "WALG_UPLOAD_CONCURRENCY": 4
}
```

```bash
# Backup commands
wal-g backup-push /var/lib/postgresql/data
wal-g backup-list
wal-g delete retain FULL 7
```

### Point-in-Time Recovery

```bash
# Restore to specific timestamp
wal-g backup-fetch /var/lib/postgresql/data LATEST
cat >> /var/lib/postgresql/data/postgresql.auto.conf << EOF
restore_command = 'wal-g wal-fetch %f %p'
recovery_target_time = '2024-01-15 14:30:00'
recovery_target_action = promote
EOF
pg_ctl -D /var/lib/postgresql/data start
```

### Base Backup Strategy

```sql
-- Create base backup with pg_basebackup
pg_basebackup -h primary -D /backup/base -U replicator -Ft -z -P -X stream

-- Schedule regular base backups (cron)
0 2 * * * pg_basebackup -h localhost -D /backup/daily/$(date +\%Y\%m\%d) -U replicator -Ft -z -P
```

---

## Monitoring and Observability

### Key Metrics to Monitor

```yaml
# Prometheus alerting rules
groups:
  - name: timescale
    rules:
      - alert: HighReplicationLag
        expr: pg_replication_lag_seconds > 30
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Replication lag is high"

      - alert: DiskSpaceLow
        expr: pg_database_size_bytes / pg_tablespace_size_bytes > 0.9
        for: 10m
        labels:
          severity: critical
        annotations:
          summary: "Database disk space is low"

      - alert: ChunkCompressionFailing
        expr: rate(timescaledb_compression_errors[5m]) > 0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Chunk compression is failing"
```

### Custom Metrics Collection

```sql
-- Create monitoring view
CREATE VIEW metrics_dashboard AS
SELECT
  schemaname,
  hypertable_name,
  count(*) as chunk_count,
  sum(pg_table_size(schemaname||'.'||table_name)) as total_size,
  sum(pg_tuple_count(schemaname||'.'||table_name::regclass)) as row_count
FROM timescaledb_information.chunks
GROUP BY 1, 2;

-- Query for monitoring
SELECT
  hypertable_name,
  pg_size_pretty(total_size) as size,
  row_count,
  chunk_count
FROM metrics_dashboard
ORDER BY total_size DESC;
```

### Grafana Dashboard Panels

```json
{
  "panels": [
    {
      "title": "Insert Rate",
      "targets": [
        {
          "expr": "rate(pg_stat_user_tuples_n_tup_ins[5m])",
          "legendFormat": "{{relname}}"
        }
      ]
    },
    {
      "title": "Query Latency",
      "targets": [
        {
          "expr": "histogram_quantile(0.95, rate(timescaledb_query_duration_seconds_bucket[5m]))",
          "legendFormat": "p95"
        }
      ]
    },
    {
      "title": "Compression Ratio",
      "targets": [
        {
          "expr": "timescaledb_compressed_size / timescaledb_uncompressed_size",
          "legendFormat": "{{hypertable}}"
        }
      ]
    }
  ]
}
```

---

## Performance Tuning

### Memory Configuration

```conf
# postgresql.conf - Memory Settings
shared_buffers = 8GB              # 25% of RAM
effective_cache_size = 24GB       # 75% of RAM
work_mem = 256MB                  # Per-operation memory
maintenance_work_mem = 2GB        # For VACUUM, CREATE INDEX
huge_pages = try                  # Enable if supported
```

### WAL Configuration

```conf
# WAL Settings
wal_buffers = 64MB
checkpoint_completion_target = 0.9
max_wal_size = 8GB
min_wal_size = 2GB
checkpoint_timeout = 15min
```

### Parallel Query

```conf
# Parallel Query Settings
max_parallel_workers_per_gather = 4
max_parallel_workers = 8
max_parallel_maintenance_workers = 4
parallel_tuple_cost = 0.01
parallel_setup_cost = 1000
```

### TimescaleDB-Specific Tuning

```conf
# TimescaleDB Settings
timescaledb.max_background_workers = 16
timescaledb.max_open_chunks_per_txn = 100
timescaledb.max_cached_chunks_per_hypertable = 50
timescaledb.compress_enable = on
```

### Index Optimization

```sql
-- Analyze index usage
SELECT
  indexrelname,
  idx_scan,
  idx_tup_read,
  idx_tup_fetch,
  pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_stat_user_indexes
WHERE relname LIKE '%chunk%'
ORDER BY idx_scan DESC;

-- Find unused indexes
SELECT
  indexrelname,
  pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_stat_user_indexes
WHERE idx_scan = 0
  AND relname LIKE '%chunk%';

-- Reindex congested indexes
REINDEX INDEX CONCURRENTLY _hyper_1_2_chunk_time_idx;
```

---

## Security

### Network Security

```conf
# pg_hba.conf - Network Access Control
# TYPE  DATABASE        USER            ADDRESS                 METHOD
host    replication     replicator      10.0.0.0/8              md5
host    all             all             10.0.0.0/8              md5
host    all             all             192.168.1.0/24          md5
hostssl all             all             0.0.0.0/0               cert
```

### SSL/TLS Configuration

```conf
# postgresql.conf - SSL Settings
ssl = on
ssl_cert_file = '/etc/ssl/certs/server.crt'
ssl_key_file = '/etc/ssl/private/server.key'
ssl_ca_file = '/etc/ssl/certs/ca.crt'
ssl_ciphers = 'HIGH:MEDIUM:+3DES:!aNULL'
ssl_prefer_server_ciphers = on
ssl_min_protocol_version = 'TLSv1.2'
```

### Role-Based Access Control

```sql
-- Create roles
CREATE ROLE readonly LOGIN PASSWORD 'secret1';
CREATE ROLE readwrite LOGIN PASSWORD 'secret2';
CREATE ROLE admin LOGIN PASSWORD 'secret3' CREATEROLE;

-- Grant permissions
GRANT CONNECT ON DATABASE timescale TO readonly, readwrite, admin;
GRANT USAGE ON SCHEMA public TO readonly, readwrite, admin;

-- Read-only access
GRANT SELECT ON ALL TABLES IN SCHEMA public TO readonly;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO readonly;

-- Read-write access
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO readwrite;
GRANT USAGE ON ALL SEQUENCES IN SCHEMA public TO readwrite;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO readwrite;

-- Admin access (TimescaleDB functions)
GRANT pg_monitor TO admin;
GRANT EXECUTE ON FUNCTION timescaledb_information TO admin;
```

### Audit Logging

```conf
# postgresql.conf - Audit Settings
log_connections = on
log_disconnections = on
log_duration = on
log_min_duration_statement = 1000
log_checkpoints = on
log_lock_waits = on
log_temp_files = 0
log_statement = 'ddl'
```

---

## Scaling Strategies

### Vertical Scaling

```sql
-- Monitor resource usage
SELECT
  pg_database_size(current_database()) as db_size,
  pg_size_pretty(pg_total_relation_size('conditions')) as table_size;

-- Check chunk distribution
SELECT
  hypertable_name,
  count(*) as chunks,
  max(pg_table_size(chunk_schema||'.'||chunk_name)) as max_chunk_size
FROM timescaledb_information.chunks
GROUP BY 1;
```

### Horizontal Scaling with Distributed Hypertables

```sql
-- Add data nodes
SELECT add_data_node('data_node_1', host => '10.0.1.1');
SELECT add_data_node('data_node_2', host => '10.0.1.2');
SELECT add_data_node('data_node_3', host => '10.0.1.3');

-- Create distributed hypertable
CREATE TABLE sensor_data (
  time TIMESTAMPTZ NOT NULL,
  sensor_id TEXT NOT NULL,
  value DOUBLE PRECISION
);

SELECT create_distributed_hypertable(
  'sensor_data',
  'time',
  'sensor_id',
  3  -- number of partitions
);
```

### Compression Scaling

```sql
-- Enable compression
ALTER TABLE sensor_data SET (
  timescaledb.compress,
  timescaledb.compress_segmentby = 'sensor_id',
  timescaledb.compress_orderby = 'time DESC'
);

-- Add compression policy
SELECT add_compression_policy('sensor_data', INTERVAL '7 days');

-- Monitor compression
SELECT
  hypertable_name,
  chunks_compressed,
  chunks_compressed_total,
  before_compression_size,
  after_compression_size,
  100 * (1 - after_compression_size::float / before_compression_size::float) as compression_ratio
FROM timescaledb_information.compression_stats;
```

### Continuous Aggregate Scaling

```sql
-- Create multi-level continuous aggregates
CREATE MATERIALIZED VIEW sensor_1min
WITH (timescaledb.continuous) AS
SELECT
  time_bucket('1 min', time) as bucket,
  sensor_id,
  avg(value) as avg_value
FROM sensor_data
GROUP BY 1, 2;

CREATE MATERIALIZED VIEW sensor_1hour
WITH (timescaledb.continuous) AS
SELECT
  time_bucket('1 hour', bucket) as bucket,
  sensor_id,
  avg(avg_value) as avg_value
FROM sensor_1min
GROUP BY 1, 2;

-- Add refresh policies
SELECT add_continuous_aggregate_policy('sensor_1min',
  start_offset => INTERVAL '1 day',
  end_offset => INTERVAL '1 min',
  schedule_interval => INTERVAL '1 min'
);

SELECT add_continuous_aggregate_policy('sensor_1hour',
  start_offset => INTERVAL '30 days',
  end_offset => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour'
);
```

---

## Troubleshooting

### Common Issues

#### Issue 1: High Memory Usage

```sql
-- Check memory consumers
SELECT
  datname,
  numbackends,
  pg_size_pretty(sum(pg_database_size(datname))) as size
FROM pg_stat_database
GROUP BY datname;

-- Check for runaway queries
SELECT
  pid,
  now() - pg_stat_activity.query_start AS duration,
  query
FROM pg_stat_activity
WHERE (now() - pg_stat_activity.query_start) > interval '1 hour'
ORDER BY duration DESC;

-- Kill long-running query
SELECT pg_terminate_backend(pid);
```

#### Issue 2: Slow Queries

```sql
-- Enable query logging
SET log_min_duration_statement = 100;

-- Check for missing indexes
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM sensor_data
WHERE time >= NOW() - INTERVAL '1 day'
  AND sensor_id = 'sensor-1';

-- Check chunk exclusion
EXPLAIN SELECT * FROM sensor_data
WHERE time >= '2024-01-01' AND time < '2024-01-02';
-- Should show only relevant chunks
```

#### Issue 3: Compression Failures

```sql
-- Check compression job history
SELECT * FROM timescaledb_information.job_history
WHERE job_id IN (SELECT job_id FROM timescaledb_config.jobs WHERE proc_name = 'policy_compression')
ORDER BY finish_time DESC
LIMIT 10;

-- Check lock contention
SELECT
  blocked_locks.pid     AS blocked_pid,
  blocking_locks.pid    AS blocking_pid,
  blocked_activity.query AS blocked_query
FROM pg_catalog.pg_locks blocked_locks
JOIN pg_catalog.pg_locks blocking_locks
  ON blocking_locks.locktype = blocked_locks.locktype
WHERE NOT blocked_locks.granted;
```

#### Issue 4: Replication Lag

```sql
-- Check replication status (primary)
SELECT
  client_addr,
  state,
  pg_wal_lsn_diff(pg_current_wal_lsn(), replay_lsn) as lag_bytes,
  sync_state
FROM pg_stat_replication;

-- Check replication status (replica)
SELECT
  pg_is_in_recovery(),
  pg_last_wal_receive_lsn(),
  pg_last_wal_replay_lsn();
```

### Diagnostic Queries

```sql
-- Database health check
SELECT
  'Database Size' as metric,
  pg_size_pretty(pg_database_size(current_database())) as value
UNION ALL
SELECT
  'Total Hypertables',
  count(*)::text
FROM timescaledb_information.hypertables
UNION ALL
SELECT
  'Total Chunks',
  count(*)::text
FROM timescaledb_information.chunks
UNION ALL
SELECT
  'Compression Ratio',
  round(100 * (1 - sum(after_compression_size)::float / sum(before_compression_size)::float), 1)::text || '%'
FROM timescaledb_information.compression_stats;
```

---

## Related Documentation

- [Rust Implementation](./rust-revision.md)
- [Storage System Guide](./storage-system-guide.md)
- [Query Optimization](./query-optimization.md)
