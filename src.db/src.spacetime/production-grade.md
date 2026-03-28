# SpacetimeDB: Production-Grade Deployment Guide

## Overview

This document covers production deployment considerations for SpacetimeDB:
- Performance optimization
- Memory management
- Monitoring and observability
- High availability
- Backup and recovery
- Scaling strategies

---

## 1. Performance Optimization

### 1.1 Memory Configuration

```rust
/// Production memory configuration
pub struct MemoryConfig {
    /// Maximum heap size for table data
    pub table_memory_limit: usize,

    /// Maximum memory for query execution
    pub query_memory_limit: usize,

    /// Commitlog buffer size
    pub commitlog_buffer_size: usize,

    /// Page size for allocations
    pub page_size: usize,
}

impl MemoryConfig {
    /// Default production config for 16GB system
    pub fn production_16gb() -> Self {
        Self {
            table_memory_limit: 8 * 1024 * 1024 * 1024,  // 8GB
            query_memory_limit: 4 * 1024 * 1024 * 1024,  // 4GB
            commitlog_buffer_size: 64 * 1024 * 1024,     // 64MB
            page_size: 8192,                              // 8KB pages
        }
    }

    /// Default production config for 64GB system
    pub fn production_64gb() -> Self {
        Self {
            table_memory_limit: 32 * 1024 * 1024 * 1024,  // 32GB
            query_memory_limit: 16 * 1024 * 1024 * 1024,  // 16GB
            commitlog_buffer_size: 128 * 1024 * 1024,     // 128MB
            page_size: 16384,                              // 16KB pages
        }
    }
}
```

### 1.2 Connection Pooling

```rust
use std::sync::Arc;
use parking_lot::Mutex;

/// Connection pool for client connections
pub struct ConnectionPool {
    /// Available connections
    available: Vec<Arc<Connection>>,

    /// Maximum connections
    max_connections: usize,

    /// Connection timeout
    timeout: Duration,
}

impl ConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        Self {
            available: Vec::with_capacity(max_connections),
            max_connections,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn acquire(&mut self) -> Result<Arc<Connection>> {
        if let Some(conn) = self.available.pop() {
            Ok(conn)
        } else if self.total_connections() < self.max_connections {
            Ok(Arc::new(Connection::new()))
        } else {
            Err("Connection pool exhausted")
        }
    }

    pub fn release(&mut self, conn: Arc<Connection>) {
        if self.available.len() < self.max_connections {
            self.available.push(conn);
        }
    }
}
```

### 1.3 Query Caching

```rust
use lru::LruCache;
use std::num::NonZeroUsize;

/// Query result cache
pub struct QueryCache {
    cache: LruCache<QueryHash, CachedResult>,
    max_memory: usize,
    current_memory: usize,
}

struct CachedResult {
    data: Vec<u8>,
    size: usize,
    created_at: Instant,
    hits: u64,
}

impl QueryCache {
    pub fn new(max_entries: usize, max_memory: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(max_entries).unwrap()),
            max_memory,
            current_memory: 0,
        }
    }

    pub fn get(&mut self, query_hash: QueryHash) -> Option<&[u8]> {
        if let Some(result) = self.cache.get(&query_hash) {
            unsafe {
                let result_ptr = self.cache.get(&query_hash).unwrap() as *const CachedResult;
                (*result_ptr).hits += 1;
            }
            Some(&result.data)
        } else {
            None
        }
    }

    pub fn insert(&mut self, query_hash: QueryHash, data: Vec<u8>) {
        let size = data.len();

        // Evict if over memory limit
        while self.current_memory + size > self.max_memory {
            if let Some((_, evicted)) = self.cache.pop_lru() {
                self.current_memory -= evicted.size;
            } else {
                break;
            }
        }

        self.cache.put(query_hash, CachedResult {
            data,
            size,
            created_at: Instant::now(),
            hits: 0,
        });

        self.current_memory += size;
    }
}
```

---

## 2. Monitoring and Observability

### 2.1 Metrics Collection

```rust
use prometheus::{Registry, Counter, Gauge, Histogram, register_counter, register_gauge, register_histogram};

/// Database metrics
pub struct DatabaseMetrics {
    /// Total queries executed
    pub queries_total: Counter,

    /// Query latency histogram
    pub query_latency: Histogram,

    /// Active connections
    pub active_connections: Gauge,

    /// Memory usage
    pub memory_usage_bytes: Gauge,

    /// Table row counts
    pub table_rows: Gauge,

    /// Commitlog size
    pub commitlog_size_bytes: Gauge,

    /// Replication lag (seconds)
    pub replication_lag_seconds: Gauge,
}

impl DatabaseMetrics {
    pub fn new(registry: &Registry) -> Result<Self> {
        Ok(Self {
            queries_total: register_counter!(
                "db_queries_total",
                "Total number of queries executed",
                registry
            )?,

            query_latency: register_histogram!(
                "db_query_latency_seconds",
                "Query latency in seconds",
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0],
                registry
            )?,

            active_connections: register_gauge!(
                "db_active_connections",
                "Number of active connections",
                registry
            )?,

            memory_usage_bytes: register_gauge!(
                "db_memory_usage_bytes",
                "Memory usage in bytes",
                registry
            )?,

            table_rows: register_gauge!(
                "db_table_rows",
                "Number of rows per table",
                &["table"],
                registry
            )?,

            commitlog_size_bytes: register_gauge!(
                "db_commitlog_size_bytes",
                "Commitlog size in bytes",
                registry
            )?,

            replication_lag_seconds: register_gauge!(
                "db_replication_lag_seconds",
                "Replication lag in seconds",
                registry
            )?,
        })
    }
}
```

### 2.2 Query Tracing

```rust
use tracing::{info, warn, error, Span, span, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Query tracing
pub struct QueryTracer {
    registry: Registry,
}

impl QueryTracer {
    pub fn init() -> Result<Self> {
        let fmt_layer = fmt::layer()
            .with_target(false)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);

        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new("info"))?;

        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();

        Ok(Self { registry: Registry::default() })
    }

    pub fn trace_query(&self, query: &str) -> QuerySpan {
        let span = span!(Level::INFO, "query", sql = query);
        let _enter = span.enter();

        QuerySpan {
            _span: span,
            start: Instant::now(),
        }
    }
}

pub struct QuerySpan {
    _span: Span,
    start: Instant,
}

impl Drop for QuerySpan {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        info!(elapsed_us = elapsed.as_micros() as u64, "Query completed");
    }
}
```

### 2.3 Health Checks

```rust
/// Health check status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl Database {
    /// Run health checks
    pub fn health_check(&self) -> HealthStatus {
        let mut checks = Vec::new();
        let mut all_healthy = true;

        // Check memory usage
        let memory_usage = self.get_memory_usage();
        if memory_usage > 0.9 {
            checks.push(HealthCheck {
                name: "memory".into(),
                status: CheckStatus::Degraded,
                message: Some(format!("Memory usage at {:.1}%", memory_usage * 100.0)),
            });
            all_healthy = false;
        } else {
            checks.push(HealthCheck {
                name: "memory".into(),
                status: CheckStatus::Healthy,
                message: None,
            });
        }

        // Check commitlog
        match self.commitlog_health() {
            Ok(_) => checks.push(HealthCheck {
                name: "commitlog".into(),
                status: CheckStatus::Healthy,
                message: None,
            }),
            Err(e) => {
                checks.push(HealthCheck {
                    name: "commitlog".into(),
                    status: CheckStatus::Unhealthy,
                    message: Some(e.to_string()),
                });
                all_healthy = false;
            }
        }

        // Check replication
        let replication_lag = self.get_replication_lag();
        if replication_lag > Duration::from_secs(60) {
            checks.push(HealthCheck {
                name: "replication".into(),
                status: CheckStatus::Degraded,
                message: Some(format!("Replication lag: {:.1}s", replication_lag.as_secs_f64())),
            });
            all_healthy = false;
        } else {
            checks.push(HealthCheck {
                name: "replication".into(),
                status: CheckStatus::Healthy,
                message: None,
            });
        }

        HealthStatus {
            healthy: all_healthy,
            checks,
        }
    }
}
```

---

## 3. High Availability

### 3.1 Failover Configuration

```rust
/// High availability configuration
pub struct HAConfig {
    /// Cluster members
    pub cluster: Vec<NodeConfig>,

    /// Failover timeout
    pub failover_timeout: Duration,

    /// Health check interval
    pub health_check_interval: Duration,

    /// Minimum quorum size
    pub quorum_size: usize,
}

pub struct NodeConfig {
    pub id: NodeId,
    pub address: SocketAddr,
    pub is_voter: bool,
    pub priority: u8,  // Higher = more likely to become leader
}

impl HAConfig {
    /// Production HA config for 3-node cluster
    pub fn production_3node() -> Self {
        Self {
            cluster: vec![
                NodeConfig { id: NodeId(1), address: "10.0.0.1:3000".parse().unwrap(), is_voter: true, priority: 1 },
                NodeConfig { id: NodeId(2), address: "10.0.0.2:3000".parse().unwrap(), is_voter: true, priority: 2 },
                NodeConfig { id: NodeId(3), address: "10.0.0.3:3000".parse().unwrap(), is_voter: true, priority: 3 },
            ],
            failover_timeout: Duration::from_secs(10),
            health_check_interval: Duration::from_secs(5),
            quorum_size: 2,  // Majority of 3
        }
    }

    /// Production HA config for 5-node cluster
    pub fn production_5node() -> Self {
        Self {
            cluster: vec![
                NodeConfig { id: NodeId(1), address: "10.0.0.1:3000".parse().unwrap(), is_voter: true, priority: 1 },
                NodeConfig { id: NodeId(2), address: "10.0.0.2:3000".parse().unwrap(), is_voter: true, priority: 2 },
                NodeConfig { id: NodeId(3), address: "10.0.0.3:3000".parse().unwrap(), is_voter: true, priority: 3 },
                NodeConfig { id: NodeId(4), address: "10.0.0.4:3000".parse().unwrap(), is_voter: true, priority: 4 },
                NodeConfig { id: NodeId(5), address: "10.0.0.5:3000".parse().unwrap(), is_voter: true, priority: 5 },
            ],
            failover_timeout: Duration::from_secs(10),
            health_check_interval: Duration::from_secs(5),
            quorum_size: 3,  // Majority of 5
        }
    }
}
```

### 3.2 Automatic Failover

```rust
pub struct FailoverManager {
    config: HAConfig,
    current_leader: Option<NodeId>,
    last_heartbeat: HashMap<NodeId, Instant>,
}

impl FailoverManager {
    pub fn check_leader_health(&mut self) -> Option<FailoverAction> {
        if let Some(leader) = self.current_leader {
            if let Some(&last_seen) = self.last_heartbeat.get(&leader) {
                if last_seen.elapsed() > self.config.failover_timeout {
                    // Leader is unresponsive, trigger election
                    return Some(FailoverAction::StartElection);
                }
            }
        }
        None
    }

    pub fn start_election(&mut self) -> NodeId {
        // Select new leader based on priority
        let mut candidates: Vec<_> = self.config.cluster
            .iter()
            .filter(|n| n.is_voter)
            .collect();

        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Find first healthy candidate
        for candidate in &candidates {
            if let Some(&last_seen) = self.last_heartbeat.get(&candidate.id) {
                if last_seen.elapsed() < self.config.failover_timeout {
                    self.current_leader = Some(candidate.id);
                    return candidate.id;
                }
            }
        }

        // No healthy candidate, return highest priority anyway
        self.current_leader = Some(candidates[0].id);
        candidates[0].id
    }
}

enum FailoverAction {
    StartElection,
}
```

---

## 4. Backup and Recovery

### 4.1 Backup Strategy

```rust
use std::fs::{File, create_dir_all};
use std::path::PathBuf;
use chrono::{DateTime, Utc};

/// Backup configuration
pub struct BackupConfig {
    /// Backup directory
    pub backup_dir: PathBuf,

    /// Full backup interval
    pub full_backup_interval: Duration,

    /// Retention period
    pub retention_days: u32,

    /// Compression enabled
    pub compression: bool,
}

impl BackupConfig {
    pub fn production() -> Self {
        Self {
            backup_dir: PathBuf::from("/var/backups/spacetimedb"),
            full_backup_interval: Duration::from_secs(86400),  // Daily
            retention_days: 30,
            compression: true,
        }
    }
}

pub struct BackupManager {
    config: BackupConfig,
    last_full_backup: Option<DateTime<Utc>>,
}

impl BackupManager {
    pub fn new(config: BackupConfig) -> Self {
        Self {
            config,
            last_full_backup: None,
        }
    }

    /// Create backup
    pub fn create_backup(&mut self, db: &Database) -> Result<BackupInfo> {
        let now = Utc::now();
        let backup_type = if self.needs_full_backup() {
            BackupType::Full
        } else {
            BackupType::Incremental
        };

        let backup_path = self.config.backup_dir.join(format!(
            "backup_{}_{}",
            now.format("%Y%m%d_%H%M%S"),
            backup_type.as_str()
        ));

        create_dir_all(&backup_path)?;

        let info = match backup_type {
            BackupType::Full => self.create_full_backup(db, &backup_path)?,
            BackupType::Incremental => self.create_incremental_backup(db, &backup_path)?,
        };

        if backup_type == BackupType::Full {
            self.last_full_backup = Some(now);
        }

        // Cleanup old backups
        self.cleanup_old_backups()?;

        Ok(info)
    }

    fn create_full_backup(&self, db: &Database, path: &Path) -> Result<BackupInfo> {
        // Snapshot all tables
        let snapshot = db.create_snapshot()?;

        // Write snapshot to file
        let snapshot_path = path.join("snapshot.bin");
        let mut file = File::create(&snapshot_path)?;

        if self.config.compression {
            let compressed = flate2::write::GzEncoder::new(
                &mut file,
                flate2::Compression::default(),
            );
            bincode::serialize_into(compressed, &snapshot)?;
        } else {
            bincode::serialize_into(&mut file, &snapshot)?;
        }

        // Copy commitlog
        let commitlog_path = path.join("commitlog.bin");
        std::fs::copy(db.commitlog_path(), &commitlog_path)?;

        Ok(BackupInfo {
            path: path.to_path_buf(),
            created_at: Utc::now(),
            backup_type: BackupType::Full,
            size: std::fs::metadata(&snapshot_path)?.len(),
        })
    }

    fn needs_full_backup(&self) -> bool {
        match self.last_full_backup {
            Some(last) => Utc::now().signed_duration_since(last) > chrono::Duration::from_std(self.config.full_backup_interval).unwrap(),
            None => true,
        }
    }

    fn cleanup_old_backups(&self) -> Result<()> {
        let cutoff = Utc::now() - chrono::Duration::days(self.config.retention_days as i64);

        for entry in std::fs::read_dir(&self.config.backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Try to parse timestamp from directory name
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(date) = chrono::NaiveDate::parse_from_str(&name[7..15], "%Y%m%d") {
                        let backup_date = DateTime::<Utc>::from_utc(date.and_hms(0, 0, 0), Utc);
                        if backup_date < cutoff {
                            std::fs::remove_dir_all(&path)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub backup_type: BackupType,
    pub size: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum BackupType {
    Full,
    Incremental,
}

impl BackupType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupType::Full => "full",
            BackupType::Incremental => "incremental",
        }
    }
}
```

### 4.2 Point-in-Time Recovery

```rust
impl Database {
    /// Restore to point in time
    pub fn restore_to_point_in_time(
        backup_path: &Path,
        target_time: DateTime<Utc>,
    ) -> Result<Self> {
        // Load backup snapshot
        let snapshot = Self::load_snapshot(backup_path.join("snapshot.bin"))?;

        // Create database from snapshot
        let mut db = Self::from_snapshot(snapshot)?;

        // Replay commitlog up to target time
        let commitlog_path = backup_path.join("commitlog.bin");
        db.replay_commitlog_to_time(&commitlog_path, target_time)?;

        Ok(db)
    }

    fn replay_commitlog_to_time(&mut self, path: &Path, target_time: DateTime<Utc>) -> Result<()> {
        let mut file = File::open(path)?;
        let mut len_buf = [0u8; 4];

        loop {
            if file.read_exact(&mut len_buf).is_err() {
                break;
            }

            let len = u32::from_be_bytes(len_buf) as usize;
            let mut bytes = vec![0u8; len];

            if file.read_exact(&mut bytes).is_err() {
                break;
            }

            let entry: CommitlogEntry = bincode::deserialize(&bytes)?;

            // Check if past target time
            if entry.timestamp > target_time {
                break;
            }

            // Apply entry
            self.apply_commitlog_entry(entry)?;
        }

        Ok(())
    }
}
```

---

## 5. Scaling Strategies

### 5.1 Read Replicas

```rust
/// Read replica configuration
pub struct ReadReplicaConfig {
    /// Primary node address
    pub primary: SocketAddr,

    /// Replica addresses
    pub replicas: Vec<SocketAddr>,

    /// Read distribution strategy
    pub read_strategy: ReadStrategy,
}

pub enum ReadStrategy {
    /// Round-robin across replicas
    RoundRobin,

    /// Least connections
    LeastConnections,

    /// Geographic proximity
    GeoProximity { client_region: String },
}

pub struct ReadRouter {
    config: ReadReplicaConfig,
    current_replica: usize,
    connection_counts: Vec<usize>,
}

impl ReadRouter {
    pub fn route_read(&mut self) -> SocketAddr {
        match self.config.read_strategy {
            ReadStrategy::RoundRobin => {
                let addr = self.config.replicas[self.current_replica];
                self.current_replica = (self.current_replica + 1) % self.config.replicas.len();
                addr
            }

            ReadStrategy::LeastConnections => {
                let min_idx = self.connection_counts
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, &count)| count)
                    .map(|(idx, _)| idx)
                    .unwrap_or(0);

                self.connection_counts[min_idx] += 1;
                self.config.replicas[min_idx]
            }

            ReadStrategy::GeoProximity { ref client_region } => {
                // Select replica in same region (simplified)
                self.config.replicas[0]
            }
        }
    }

    pub fn release_connection(&mut self, addr: SocketAddr) {
        if let Some(idx) = self.config.replicas.iter().position(|&a| a == addr) {
            self.connection_counts[idx] = self.connection_counts[idx].saturating_sub(1);
        }
    }
}
```

### 5.2 Sharding

```rust
/// Sharding configuration
pub struct ShardingConfig {
    /// Number of shards
    pub num_shards: usize,

    /// Shard key column
    pub shard_key: String,

    /// Hash function
    pub hash_function: HashFunction,
}

pub enum HashFunction {
    Murmur3,
    XxHash,
    Blake3,
}

pub struct ShardRouter {
    config: ShardingConfig,
    shards: HashMap<u64, SocketAddr>,  // shard_id -> node
}

impl ShardRouter {
    pub fn route(&self, shard_key: &DbValue) -> SocketAddr {
        let hash = self.compute_hash(shard_key);
        let shard_id = hash % self.config.num_shards as u64;
        self.shards[&shard_id]
    }

    fn compute_hash(&self, key: &DbValue) -> u64 {
        use xxhash_rust::xxh3::xxh3_64;

        match key {
            DbValue::String(s) => xxh3_64(s.as_bytes()),
            DbValue::I64(i) => xxh3_64(&i.to_le_bytes()),
            DbValue::U64(i) => xxh3_64(&i.to_le_bytes()),
            _ => xxh3_64(&format!("{:?}", key).into_bytes()),
        }
    }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial production-grade guide created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
