---
title: "Production-Grade Content-Addressed Data: Operating at Scale"
subtitle: "Building, deploying, and operating content-addressed systems in production environments"
based_on: "CID Router architecture with production enhancements"
level: "Advanced - Production engineering considerations"
---

# Production-Grade Content-Addressed Data: Operations and Scaling Guide

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
┌─────────────────────────────────────────────────────────────┐
│                  Single CID Router Node                      │
├─────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────┐  │
│  │              HTTP API Server (Axum)                    │  │
│  │              Port: 8080                                │  │
│  └─────────────────────┬─────────────────────────────────┘  │
│                        │                                     │
│  ┌─────────────────────▼─────────────────────────────────┐  │
│  │              CID Router Core                           │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │  Indexer (Background)                           │  │  │
│  │  │  Interval: 3600s                                │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │  SQLite Database                                │  │  │
│  │  │  ~/.local/share/cid-router/db.sqlite            │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────┘  │
│                        │                                     │
│  ┌─────────────────────▼─────────────────────────────────┐  │
│  │              Content Providers (CRPs)                  │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │  │
│  │  │   Iroh      │  │   Azure     │  │   Local     │   │  │
│  │  │   (P2P)     │  │  (Cloud)    │  │   (FS)      │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘   │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**Distributed DHT Cluster (Production):**
```
┌─────────────────────────────────────────────────────────────────┐
│                         LOAD BALANCER                            │
│              (nginx / HAProxy / Cloud LB)                        │
└───────────────────────────┬─────────────────────────────────────┘
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
  ┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐
  │  Node-1     │   │  Node-2     │   │  Node-3     │
  │  (Primary)  │   │  (Replica)  │   │  (Replica)  │
  │  DHT Peer   │   │  DHT Peer   │   │  DHT Peer   │
  └──────┬──────┘   └──────┬──────┘   └──────┬──────┘
         │                 │                 │
         └─────────────────┴─────────────────┘
                           │
         ┌─────────────────▼─────────────────┐
         │       Shared Storage Layer         │
         │  ┌─────────────┐ ┌─────────────┐  │
         │  │  Azure Blob │ │   Iroh      │  │
         │  │  Storage    │ │   Store     │  │
         │  └─────────────┘ └─────────────┘  │
         └───────────────────────────────────┘
```

**Pinning Service Cluster (Multi-tenant):**
```
┌─────────────────────────────────────────────────────────────────┐
│                         API GATEWAY                              │
│              (Kong / Traefik / Cloud API Gateway)                │
└───────────────────────────┬─────────────────────────────────────┘
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
  ┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐
  │   Tenant-A  │   │   Tenant-B  │   │   Tenant-C  │
  │   Cluster   │   │   Cluster   │   │   Cluster   │
  │  ┌────────┐ │   │  ┌────────┐ │   │  ┌────────┐ │
  │  │ Router │ │   │  │ Router │ │   │  │ Router │ │
  │  └───┬────┘ │   │  └───┬────┘ │   │  └───┬────┘ │
  └──────┼──────┘   └──────┼──────┘   └──────┼──────┘
         │                 │                 │
         └─────────────────┴─────────────────┘
                           │
         ┌─────────────────▼─────────────────┐
         │     Object Storage (Multi-tenant) │
         │  ┌─────────┐ ┌─────────┐ ┌──────┐ │
         │  │Bucket-A │ │Bucket-B │ │ ... │ │
         │  └─────────┘ └─────────┘ └──────┘ │
         └───────────────────────────────────┘
```

### 1.2 Component Sizing Recommendations

| Component | Small | Medium | Large | XLarge |
|-----------|-------|--------|-------|--------|
| **CPU** | 2 cores | 4 cores | 8 cores | 16+ cores |
| **RAM** | 4 GB | 8 GB | 16 GB | 64+ GB |
| **Storage (Local)** | 50 GB SSD | 200 GB SSD | 500 GB NVMe | 2+ TB NVMe |
| **Network** | 1 Gbps | 1 Gbps | 10 Gbps | 25+ Gbps |
| **Use Case** | Dev/Test | Small team | Production | Enterprise |

**CID Router Specific Sizing:**

| Metric | Small | Medium | Large | XLarge |
|--------|-------|--------|-------|--------|
| Max CIDs indexed | 100K | 1M | 10M | 100M+ |
| Max concurrent requests | 50 | 200 | 1000 | 5000+ |
| SQLite DB size | < 1 GB | < 10 GB | < 100 GB | 100+ GB |
| Blob store capacity | 100 GB | 1 TB | 10 TB | 100+ TB |

### 1.3 Production Configuration Template

```toml
# ~/.local/share/cid-router/server.toml

# Server Configuration
port = 8080
bind_address = "0.0.0.0"

# Production Authentication
[auth]
type = "eqty_jwt"
jwks_url = "https://auth.example.com/.well-known/jwks.json"

# Iroh Provider (Primary P2P)
[[providers]]
type = "iroh"
path = "/var/lib/cid-router/blobs"
writeable = true

# Azure Provider (Cloud Backup)
[[providers]]
type = "azure"
account = "${AZURE_STORAGE_ACCOUNT}"
container = "production-data"
filter = { directory = "cid-router/" }

# Local Provider (Fast Cache)
[[providers]]
type = "local"
path = "/mnt/nvme/cid-cache"
writeable = true
```

---

## 2. High Availability Design

### 2.1 Replication Strategies

**Database Replication (SQLite WAL Mode):**

```rust
// Enable WAL mode for concurrent reads
db.conn.execute(
    "PRAGMA journal_mode = WAL;
     PRAGMA synchronous = NORMAL;
     PRAGMA wal_autocheckpoint = 1000;",
    [],
)?;

// Configure busy timeout for concurrent access
db.conn.execute("PRAGMA busy_timeout = 5000;", [])?;
```

**Provider Redundancy:**

```
Primary Content Flow:
┌──────────┐     ┌──────────┐     ┌──────────┐
│  Client  │────▶│  Router  │────▶│  Iroh-1  │
└──────────┘     └──────────┘     └────┬─────┘
       │                               │
       │ Fallback                      │
       └───────────────────────────────┘
                               ┌───────▼───────┐
                               │   Iroh-2      │
                               │   (Replica)   │
                               └───────────────┘

Upload Flow (Multi-provider):
┌──────────┐     ┌──────────┐
│  Client  │────▶│  Router  │
└──────────┘     └────┬─────┘
                      │
         ┌────────────┼────────────┐
         │            │            │
  ┌──────▼──────┐ ┌──▼────────┐ ┌─▼──────────┐
  │   Iroh-1    │ │  Azure    │ │   Local    │
  │  (Primary)  │ │ (Backup)  │ │  (Cache)   │
  └─────────────┘ └───────────┘ └────────────┘
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
  │  ✅ Active  │           │  ⏸ Waiting  │
  └─────────────┘           └──────┬──────┘
                                   │
                          ┌────────▼────────┐
                          │ Virtual IP / DNS │
                          │ cid-router.local │
                          └─────────────────┘
```

**Health Check Implementation:**

```bash
#!/bin/bash
# health-check.sh

# Check HTTP endpoint
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/v1/status)
if [ "$HTTP_STATUS" != "200" ]; then
    echo "HTTP health check failed"
    exit 1
fi

# Check database connectivity
if ! cid-router exec "SELECT 1;" > /dev/null 2>&1; then
    echo "Database health check failed"
    exit 1
fi

# Check provider connectivity
for provider in iroh azure local; do
    if ! cid-router check-provider "$provider"; then
        echo "Provider $provider health check failed"
        exit 1
    fi
done

echo "All health checks passed"
exit 0
```

**Automatic Failover Script:**

```bash
#!/bin/bash
# failover.sh

PRIMARY="cid-router-primary"
STANDBY="cid-router-standby"
VIRTUAL_IP="10.0.1.100"

check_primary() {
    curl -sf "http://$PRIMARY:8080/v1/status" > /dev/null
}

if ! check_primary; then
    echo "Primary failed, initiating failover..."

    # Wait for standby to be ready
    until curl -sf "http://$STANDBY:8080/v1/status" > /dev/null; do
        sleep 1
    done

    # Update virtual IP (using keepalived or similar)
    ip addr add $VIRTUAL_IP/32 dev eth0

    # Update DNS (if using dynamic DNS)
    update_dns "cid-router.example.com" "$STANDBY"

    # Alert on-call team
    send_alert "CID Router failover completed to $STANDBY"

    # Log the failover event
    logger "CID Router failover: $PRIMARY -> $STANDBY"
fi
```

### 2.3 Content Redundancy

**Multi-Provider Pinning:**

```rust
// Upload to multiple providers for redundancy
async fn pin_content_redundant(
    data: &[u8],
    providers: &[Arc<dyn Crp>],
    min_replicas: usize,
) -> Result<Vec<Route>> {
    let hash = blake3::hash(data);
    let cid = blake3_hash_to_cid(hash.into(), Codec::Raw);
    let mut routes = Vec::new();

    // Write to all eligible providers
    for provider in providers {
        if let Some(writer) = provider.capabilities().blob_writer {
            match writer.put_blob(None, &cid, data).await {
                Ok(_) => {
                    // Create route record
                    let route = Route::builder(provider)
                        .cid(cid)
                        .size(data.len() as u64)
                        .build(&ctx)?;
                    routes.push(route);
                }
                Err(e) => {
                    log::warn!("Failed to pin to {:?}: {}", provider.provider_id(), e);
                }
            }
        }
    }

    // Verify minimum replicas
    if routes.len() < min_replicas {
        return Err(anyhow!(
            "Only {} replicas, minimum required: {}",
            routes.len(),
            min_replicas
        ));
    }

    Ok(routes)
}
```

---

## 3. Scaling Strategies

### 3.1 DHT Scaling (Kademlia Bucket Management)

**Routing Table Structure:**

```
Node ID: 0x1234567890abcdef (160-bit space)

K-Buckets organized by XOR distance:

Bucket 0   (distance 1-1):    Nodes 0xxxxxxxxxxxxxxx
Bucket 1   (distance 2-3):    Nodes 10xxxxxxxxxxxxxx
Bucket 2   (distance 4-7):    Nodes 110xxxxxxxxxxxxx
...
Bucket 159 (distance 2^159):  Nodes 0111111...

Each bucket holds up to k=20 nodes (default)
```

**Bucket Management Strategy:**

```rust
// Kademlia bucket configuration
pub struct KademliaConfig {
    /// Bucket size (k-parameter)
    pub bucket_size: usize,           // Default: 20

    /// Concurrent queries (alpha-parameter)
    pub concurrent_queries: usize,    // Default: 3

    /// Pending entries per bucket
    pub pending_entries: usize,       // Default: 10

    /// Bucket refresh interval
    pub refresh_interval: Duration,   // Default: 5 minutes

    /// Connection timeout
    pub connection_timeout: Duration, // Default: 10 seconds
}

// Production tuning for large networks
let config = KademliaConfig {
    bucket_size: 40,              // Larger buckets for better connectivity
    concurrent_queries: 5,        // More parallel queries
    pending_entries: 20,          // More pending entries
    refresh_interval: Duration::from_secs(120),  // Faster refresh
    connection_timeout: Duration::from_secs(5),  // Faster timeout
};
```

**Peer Routing Table Optimization:**

```rust
// Custom peer scoring for better routing
pub struct PeerScore {
    latency_ms: u64,
    success_rate: f64,
    last_seen: Instant,
    content_count: u64,
}

impl PeerScorer {
    pub fn calculate_score(&self, peer: &PeerInfo) -> f64 {
        let latency_score = 1.0 / (1.0 + peer.latency_ms as f64 / 100.0);
        let freshness_score = 1.0 / (1.0 + peer.last_seen.elapsed().as_secs() as f64 / 3600.0);

        latency_score * 0.4 +
        peer.success_rate * 0.3 +
        freshness_score * 0.2 +
        (peer.content_count as f64).log10() * 0.1
    }

    pub fn evict_low_score_peers(&self, bucket: &mut KBucket, threshold: f64) {
        bucket.retain(|peer| self.calculate_score(peer) >= threshold);
    }
}
```

### 3.2 Bitswap Parallelism and Session Management

**Bitswap Session Configuration:**

```rust
pub struct BitswapConfig {
    /// Maximum want-list size
    pub max_want_list_size: usize,     // Default: 1024

    /// Target block size for requests
    pub target_block_size: usize,      // Default: 16384 (16KB)

    /// Max concurrent fetches per session
    pub max_concurrent_fetches: usize, // Default: 10

    /// Timeout for individual block requests
    pub block_request_timeout: Duration,

    /// Number of peers to broadcast wants to
    pub want_broadcast_peers: usize,   // Default: 3
}

// Production configuration for high throughput
let config = BitswapConfig {
    max_want_list_size: 2048,
    target_block_size: 65536,         // 64KB blocks
    max_concurrent_fetches: 20,
    block_request_timeout: Duration::from_secs(30),
    want_broadcast_peers: 5,
};
```

**Session Manager:**

```rust
pub struct SessionManager {
    active_sessions: DashMap<SessionId, Session>,
    max_sessions: usize,
    session_timeout: Duration,
}

impl SessionManager {
    pub fn create_session(&self, cid: Cid, peers: Vec<PeerId>) -> SessionId {
        let session_id = SessionId::new();

        let session = Session {
            cid,
            peers,
            want_list: HashSet::new(),
            received_blocks: HashMap::new(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
        };

        self.active_sessions.insert(session_id, session);
        session_id
    }

    pub fn cleanup_stale_sessions(&self) {
        let now = Instant::now();
        self.active_sessions.retain(|_, session| {
            now.duration_since(session.last_activity) < self.session_timeout
        });
    }
}
```

### 3.3 Content Routing at Scale (CRP Strategies)

**CRP Selection Strategy:**

```rust
pub enum CrpSelectionStrategy {
    /// Route to first available provider
    FirstAvailable,

    /// Route to provider with lowest latency
    LowestLatency,

    /// Route to provider with most content
    MostContent,

    /// Load balance across providers
    LoadBalanced {
        strategy: LoadBalanceAlgorithm,
    },

    /// Route based on content type
    ContentBased {
        rules: Vec<ContentRoutingRule>,
    },
}

pub enum LoadBalanceAlgorithm {
    RoundRobin,
    LeastConnections,
    Weighted { weights: HashMap<ProviderId, f64> },
}
```

**CRP Connection Pool:**

```rust
pub struct CrpConnectionPool {
    pools: DashMap<ProviderType, Pool<CrpConnection>>,
    max_connections_per_provider: usize,
    connection_timeout: Duration,
}

impl CrpConnectionPool {
    pub async fn get_connection(&self, provider: &ProviderId) -> Result<CrpConnection> {
        let pool = self.pools.get(provider.type()).unwrap();

        // Get connection with timeout
        tokio::time::timeout(
            self.connection_timeout,
            pool.get()
        ).await??

        Ok(connection)
    }

    pub fn health_check_all(&self) -> Vec<(ProviderId, bool)> {
        self.pools.iter().map(|entry| {
            let provider = entry.key();
            let healthy = entry.value().state().connections > 0;
            (provider.clone(), healthy)
        }).collect()
    }
}
```

---

## 4. Performance Tuning

### 4.1 Block Caching Strategies

**Multi-Level Cache Architecture:**

```
┌─────────────────────────────────────────────────────────────┐
│                    Request Flow                              │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│  L1: In-Memory Cache (moka/redis)       │
│  - Hot blocks (< 1MB)                   │
│  - TTL: 5 minutes                       │
│  - Max size: 2 GB                       │
│  - Hit rate target: > 80%               │
└────────────────┬────────────────────────┘
                 │ Miss
                 ▼
┌─────────────────────────────────────────┐
│  L2: Local SSD Cache (NVMe)             │
│  - Warm blocks                          │
│  - LRU eviction                          │
│  - Max size: 100 GB                     │
│  - Hit rate target: > 60%               │
└────────────────┬────────────────────────┘
                 │ Miss
                 ▼
┌─────────────────────────────────────────┐
│  L3: Provider Storage (Azure/Iroh)      │
│  - Cold storage                         │
│  - Fetch and populate L1/L2             │
└─────────────────────────────────────────┘
```

**In-Memory Cache Configuration:**

```rust
use moka::future::Cache;

pub struct BlockCache {
    cache: Cache<Cid, Bytes>,
    stats: Arc<CacheStats>,
}

impl BlockCache {
    pub fn new(max_size: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_size)
            .time_to_live(ttl)
            .time_to_idle(Duration::from_secs(60))
            .weigher(|_key, value: &Bytes| -> u32 {
                value.len() as u32
            })
            .logger(Log::new())
            .build();

        Self {
            cache,
            stats: Arc::new(CacheStats::new()),
        }
    }

    pub async fn get(&self, cid: &Cid) -> Option<Bytes> {
        let start = Instant::now();
        let result = self.cache.get(cid).await;
        self.stats.record_get(start.elapsed(), result.is_some());
        result
    }

    pub async fn insert(&self, cid: Cid, data: Bytes) {
        self.cache.insert(cid, data).await;
    }
}
```

**Cache Warming Strategy:**

```rust
// Proactively cache linked blocks
async fn warm_linked_blocks(
    cache: &BlockCache,
    root_cid: Cid,
    max_depth: usize,
) -> Result<usize> {
    let mut warmed = 0;
    let mut queue = VecDeque::new();
    queue.push_back((root_cid, 0));

    while let Some((cid, depth)) = queue.pop_front() {
        if depth > max_depth {
            continue;
        }

        // Fetch and cache the block
        let block = fetch_block(cid).await?;
        cache.insert(cid, block.clone()).await;
        warmed += 1;

        // Queue linked CIDs
        for linked_cid in extract_links(&block) {
            queue.push_back((linked_cid, depth + 1));
        }
    }

    Ok(warmed)
}
```

### 4.2 Connection Pooling for libp2p

**libp2p Connection Pool:**

```rust
pub struct Libp2pConnectionPool {
    swarm: Arc<Mutex<Swarm<Behaviour>>>,
    connection_limits: ConnectionLimits,
    idle_connection_timeout: Duration,
}

impl Libp2pConnectionPool {
    pub fn new(config: PoolConfig) -> Result<Self> {
        let connection_limits = ConnectionLimits::default()
            .with_max_pending_incoming(Some(config.max_pending_incoming))
            .with_max_pending_outgoing(Some(config.max_pending_outgoing))
            .with_max_established_incoming(Some(config.max_established_incoming))
            .with_max_established_outgoing(Some(config.max_established_outgoing))
            .with_max_established_per_peer(Some(config.max_per_peer));

        // Configure idle connection timeout
        let idle_timeout = config.idle_connection_timeout;

        Ok(Self {
            swarm: Arc::new(Mutex::new(swarm)),
            connection_limits,
            idle_connection_timeout: idle_timeout,
        })
    }

    pub async fn dial(&self, peer: PeerId, address: Multiaddr) -> Result<()> {
        self.swarm.lock().await.dial(address)?;
        Ok(())
    }
}

// Production connection limits
let pool_config = PoolConfig {
    max_pending_incoming: 100,
    max_pending_outgoing: 100,
    max_established_incoming: 500,
    max_established_outgoing: 500,
    max_per_peer: 10,
    idle_connection_timeout: Duration::from_secs(30),
};
```

### 4.3 Batch Operations for Bulk Imports

**Batch Insert with Transaction:**

```rust
pub async fn batch_insert_routes(
    db: &Db,
    routes: Vec<Route>,
    batch_size: usize,
) -> Result<usize> {
    let mut inserted = 0;

    for chunk in routes.chunks(batch_size) {
        // Single transaction for batch
        db.transaction(|tx| {
            for route in chunk {
                tx.execute(
                    "INSERT OR IGNORE INTO routes (...) VALUES (...)",
                    params![...],
                )?;
                inserted += 1;
            }
            Ok(())
        }).await?;
    }

    Ok(inserted)
}

// Recommended batch sizes
const BATCH_SIZE_SMALL: usize = 100;   // For interactive operations
const BATCH_SIZE_MEDIUM: usize = 1000; // For background jobs
const BATCH_SIZE_LARGE: usize = 10000; // For bulk imports
```

**Parallel Batch Processing:**

```rust
pub async fn parallel_batch_process(
    items: Vec<ImportItem>,
    concurrency: usize,
) -> Result<ProcessResult> {
    let results = futures::stream::iter(items)
        .map(|item| process_item(item))
        .buffered(concurrency)
        .collect::<Vec<_>>()
        .await;

    let (successes, failures): (Vec<_>, Vec<_>) =
        results.into_iter().partition(|r| r.is_ok());

    Ok(ProcessResult {
        success_count: successes.len(),
        failure_count: failures.len(),
        failures,
    })
}

// Production concurrency settings
const CONCURRENCY_LOW: usize = 4;    // For I/O bound with large items
const CONCURRENCY_MEDIUM: usize = 16; // For balanced workloads
const CONCURRENCY_HIGH: usize = 64;  // For small, fast operations
```

### 4.4 Network Optimization

**Multiplexing Configuration:**

```rust
// Yamux multiplexing configuration
let yamux_config = yamux::Config::default()
    .set_max_num_streams(256)           // Max concurrent streams
    .set_receive_window_size(16 * 1024 * 1024)  // 16MB receive window
    .set_connection_window_size(64 * 1024 * 1024) // 64MB connection window
    .set_max_buffer_size(8 * 1024 * 1024)  // 8MB buffer per stream
    .set_split_send_size(64 * 1024)     // 64KB send splits
    .set_split_recv_size(64 * 1024);    // 64KB receive splits
```

**Compression Configuration:**

```rust
// zstd compression for libp2p
let compress_config = libp2p::zstd::Config::default()
    .with_compression_level(3)  // Fast compression (1-22, 3 is good balance)
    .with_decompression_size_limit(16 * 1024 * 1024); // 16MB limit

// When to use compression:
// - Content > 1KB: Always use compression
// - Content 100B-1KB: Use for text/JSON, skip for binary
// - Content < 100B: Skip compression (overhead > benefit)
```

**TCP Tuning:**

```bash
# Linux sysctl tuning for high-throughput content delivery
net.core.rmem_max = 134217728      # 128MB max receive buffer
net.core.wmem_max = 134217728      # 128MB max send buffer
net.ipv4.tcp_rmem = 4096 87380 134217728  # TCP receive buffer
net.ipv4.tcp_wmem = 4096 65536 134217728  # TCP send buffer
net.ipv4.tcp_congestion_control = bbr     # Use BBR congestion control
net.core.netdev_max_backlog = 5000        # NIC queue size
net.ipv4.tcp_max_syn_backlog = 8192       # SYN queue size
```

---

## 5. Backup and Recovery

### 5.1 Pinning Strategies for Data Persistence

**Pinning Policy Types:**

```rust
pub enum PinningPolicy {
    /// Pin content indefinitely
    Permanent,

    /// Pin for specified duration
    Temporary { duration: Duration },

    /// Pin with automatic garbage collection
    GarbageCollected {
        min_pins: usize,
        max_age: Duration,
    },

    /// Pin based on access frequency
    AccessBased {
        min_access_count: usize,
        window: Duration,
    },
}

// Production pinning configuration
let policy = PinningPolicy::GarbageCollected {
    min_pins: 3,        // Keep at least 3 copies
    max_age: Duration::from_days(30),  // GC after 30 days
};
```

**Pin Queue Management:**

```rust
pub struct PinQueue {
    queue: PriorityQueue<Cid, PinPriority>,
    in_progress: HashSet<Cid>,
    completed: LruCache<Cid, PinResult>,
}

impl PinQueue {
    pub fn enqueue(&mut self, cid: Cid, priority: PinPriority) {
        self.queue.push(cid, priority);
    }

    pub fn dequeue(&mut self) -> Option<(Cid, PinPriority)> {
        self.queue.pop()
    }

    pub fn mark_in_progress(&mut self, cid: Cid) {
        self.in_progress.insert(cid);
    }

    pub fn mark_completed(&mut self, cid: Cid, result: PinResult) {
        self.in_progress.remove(&cid);
        self.completed.put(cid, result);
    }
}
```

### 5.2 Disaster Recovery for Pinning Services

**Backup Strategy:**

```
┌─────────────────────────────────────────────────────────────┐
│                  Backup Architecture                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Primary Site                    Disaster Recovery Site      │
│  ┌─────────────────┐            ┌─────────────────┐         │
│  │  CID Router     │            │  CID Router     │         │
│  │  (Active)       │───────────▶│  (Standby)      │         │
│  │                 │  Async     │                 │         │
│  │  SQLite DB      │  Replication│  SQLite DB     │         │
│  │  + Blob Store   │            │  + Blob Store   │         │
│  └─────────────────┘            └─────────────────┘         │
│                                                              │
│  Backup Schedule:                                            │
│  - Database: Continuous WAL shipping                         │
│  - Blobs: Async replication on write                         │
│  - Config: Daily snapshot to S3                              │
│                                                              │
│  RPO Target: < 5 minutes                                     │
│  RTO Target: < 30 minutes                                    │
└─────────────────────────────────────────────────────────────┘
```

**Disaster Recovery Runbook:**

```bash
#!/bin/bash
# disaster-recovery.sh

set -e

PRIMARY_SITE="us-east-1"
DR_SITE="us-west-2"

echo "=== CID Router Disaster Recovery ==="

# Step 1: Assess damage
echo "Step 1: Assessing primary site..."
if curl -sf "https://primary.cid-router.example.com/v1/status" > /dev/null; then
    echo "Primary site is healthy. No DR needed."
    exit 0
fi

# Step 2: Verify DR site is ready
echo "Step 2: Verifying DR site..."
DR_STATUS=$(curl -sf "https://dr.cid-router.example.com/v1/status" | jq -r '.status')
if [ "$DR_STATUS" != "standby" ]; then
    echo "DR site not in standby state: $DR_STATUS"
    exit 1
fi

# Step 3: Promote DR to primary
echo "Step 3: Promoting DR site..."
curl -X POST "https://dr.cid-router.example.com/admin/promote" \
    -H "Authorization: Bearer $ADMIN_TOKEN"

# Step 4: Update DNS
echo "Step 4: Updating DNS..."
aws route53 change-resource-record-sets \
    --hosted-zone-id "$ZONE_ID" \
    --change-batch '{
        "Changes": [{
            "Action": "UPSERT",
            "ResourceRecordSet": {
                "Name": "cid-router.example.com",
                "Type": "A",
                "TTL": 60,
                "ResourceRecords": [{"Value": "'$DR_IP'"}]
            }
        }]
    }'

# Step 5: Verify failover
echo "Step 5: Verifying failover..."
sleep 30  # Wait for DNS propagation
curl -sf "https://cid-router.example.com/v1/status" || exit 1

echo "=== Disaster Recovery Completed ==="
```

### 5.3 Data Integrity Verification

**Merkle Proof Verification:**

```rust
pub fn verify_merkle_proof(
    cid: &Cid,
    proof: Vec<MerkleProofNode>,
    root: &Cid,
) -> bool {
    let mut current_hash = cid.hash().digest().to_vec();

    for node in proof {
        match node {
            MerkleProofNode::Left(sibling_hash) => {
                current_hash = hash_pair(&sibling_hash, &current_hash);
            }
            MerkleProofNode::Right(sibling_hash) => {
                current_hash = hash_pair(&current_hash, &sibling_hash);
            }
        }
    }

    &current_hash == root.hash().digest()
}

// Periodic integrity audit
pub async fn audit_content_integrity(
    db: &Db,
    sample_rate: f64,
) -> Result<AuditReport> {
    let routes = db.list_all_routes().await?;
    let mut report = AuditReport::new();

    for route in routes {
        if rand::random::<f64>() > sample_rate {
            continue;  // Skip based on sample rate
        }

        match verify_route_integrity(&route).await {
            Ok(true) => report.verified += 1,
            Ok(false) => {
                report.corrupted += 1;
                report.corrupted_cids.push(route.cid);
            }
            Err(e) => {
                report.unreachable += 1;
                report.errors.push((route.cid, e));
            }
        }
    }

    Ok(report)
}
```

**Integrity Check Scheduler:**

```rust
pub struct IntegrityChecker {
    db: Arc<Db>,
    schedule: IntegritySchedule,
}

pub struct IntegritySchedule {
    full_audit: CronExpression,      // Monthly full audit
    sampled_audit: CronExpression,   // Weekly 10% sample
    critical_audit: CronExpression,  // Daily critical content
}

impl IntegrityChecker {
    pub async fn run_scheduled_checks(&self) {
        loop {
            let now = Utc::now();

            if self.schedule.full_audit.matches(&now) {
                self.run_full_audit(1.0).await;
            }

            if self.schedule.sampled_audit.matches(&now) {
                self.run_full_audit(0.1).await;  // 10% sample
            }

            if self.schedule.critical_audit.matches(&now) {
                self.run_critical_audit().await;
            }

            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    }
}
```

---

## 6. Monitoring and Observability

### 6.1 Kademlia Metrics

**Key DHT Metrics:**

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `kademlia_bucket_size` | Nodes per bucket | < 5 for >50% buckets |
| `kademlia_query_latency_seconds` | Query response time | p99 > 2s |
| `kademlia_routing_table_size` | Total known peers | < 100 |
| `kademlia_peer_discovery_rate` | New peers per minute | < 1/min |
| `kademlia_failed_queries` | Failed query count | > 10/min |

**Prometheus Metrics Export:**

```rust
use prometheus::{register_histogram_vec, register_gauge_vec, HistogramVec, GaugeVec};

pub struct KademliaMetrics {
    bucket_size: GaugeVec,
    query_latency: HistogramVec,
    routing_table_size: GaugeVec,
    failed_queries: GaugeVec,
}

impl KademliaMetrics {
    pub fn new(registry: &Registry) -> Result<Self> {
        Ok(Self {
            bucket_size: register_gauge_vec!(
                "kademlia_bucket_size",
                "Number of peers in each K-bucket",
                &["bucket_index"]
            )?,
            query_latency: register_histogram_vec!(
                "kademlia_query_latency_seconds",
                "Latency of Kademlia queries",
                &["query_type"],
                vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0]
            )?,
            routing_table_size: register_gauge_vec!(
                "kademlia_routing_table_size",
                "Total number of peers in routing table",
                &[]
            )?,
            failed_queries: register_gauge_vec!(
                "kademlia_failed_queries_total",
                "Total number of failed Kademlia queries",
                &["error_type"]
            )?,
        })
    }

    pub fn record_query(&self, query_type: &str, duration: Duration, success: bool) {
        self.query_latency
            .with_label_values(&[query_type])
            .observe(duration.as_secs_f64());

        if !success {
            self.failed_queries
                .with_label_values(&["timeout"])
                .inc();
        }
    }
}
```

### 6.2 Bitswap Transfer Metrics

**Bitswap Metrics:**

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `bitswap_want_list_size` | Current want-list size | > 500 |
| `bitswap_blocks_received` | Blocks received per second | < 10/s |
| `bitswap_bytes_received` | Bytes received per second | Monitor trend |
| `bitswap_session_count` | Active sessions | > 100 |
| `bitswap_session_duration` | Session duration | p99 > 60s |
| `bitswap_duplicate_wants` | Duplicate want requests | > 20% |

**Session Health Monitoring:**

```rust
pub struct SessionHealth {
    active_sessions: Gauge,
    session_duration: HistogramVec,
    blocks_per_session: Histogram,
    stale_sessions: Gauge,
}

impl SessionHealth {
    pub fn check_session_health(&self) -> SessionHealthReport {
        let now = Instant::now();
        let mut report = SessionHealthReport::default();

        for session in self.active_sessions.iter() {
            let age = now.duration_since(session.created_at);
            let idle = now.duration_since(session.last_activity);

            if idle > Duration::from_secs(300) {
                report.stale_sessions += 1;
            }

            if age > Duration::from_secs(3600) {
                report.long_running_sessions += 1;
            }
        }

        report
    }
}
```

### 6.3 Storage Utilization Tracking

**Storage Metrics:**

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `storage_bytes_total` | Total bytes stored | > 85% capacity |
| `storage_bytes_by_provider` | Bytes per provider | Imbalance > 2x |
| `storage_objects_total` | Total objects (CIDs) | Monitor growth |
| `storage_gc_reclaim_rate` | Bytes reclaimed by GC | < expected |
| `storage_write_rate` | Bytes written per second | Spike detection |
| `storage_read_rate` | Bytes read per second | Spike detection |

**Capacity Planning Dashboard:**

```rust
pub struct StorageCapacity {
    total_capacity: u64,
    used_capacity: u64,
    projected_growth: f64,  // bytes per day
}

impl StorageCapacity {
    pub fn days_until_full(&self) -> u64 {
        let remaining = self.total_capacity - self.used_capacity;
        (remaining as f64 / self.projected_growth) as u64
    }

    pub fn utilization_percent(&self) -> f64 {
        self.used_capacity as f64 / self.total_capacity as f64 * 100.0
    }

    pub fn alert_threshold(&self, threshold: f64) -> bool {
        self.utilization_percent() >= threshold
    }
}

// Capacity alerting
let capacity = get_storage_capacity().await?;
if capacity.alert_threshold(85.0) {
    send_alert(format!(
        "Storage at {:.1}% capacity. {} days until full.",
        capacity.utilization_percent(),
        capacity.days_until_full()
    ));
}
```

### 6.4 Grafana Dashboard Configuration

**Dashboard JSON Export:**

```json
{
  "dashboard": {
    "title": "CID Router Production",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "rate(http_requests_total{job=\"cid-router\"}[5m])",
            "legendFormat": "{{method}} {{path}}"
          }
        ]
      },
      {
        "title": "Request Latency (p50, p90, p99)",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "p50"
          },
          {
            "expr": "histogram_quantile(0.90, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "p90"
          },
          {
            "expr": "histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "p99"
          }
        ]
      },
      {
        "title": "DHT Routing Table Size",
        "targets": [
          {
            "expr": "kademlia_routing_table_size",
            "legendFormat": "Peers"
          }
        ]
      },
      {
        "title": "Storage Utilization",
        "targets": [
          {
            "expr": "storage_bytes_total / storage_capacity_bytes * 100",
            "legendFormat": "Utilization %"
          }
        ]
      },
      {
        "title": "Bitswap Active Sessions",
        "targets": [
          {
            "expr": "bitswap_session_count",
            "legendFormat": "Sessions"
          }
        ]
      }
    ]
  }
}
```

---

## 7. Security Hardening

### 7.1 Content Verification (CID Validation)

**CID Validation Pipeline:**

```rust
pub async fn validate_and_store(
    data: &[u8],
    expected_cid: &Cid,
) -> Result<ValidationReport> {
    let mut report = ValidationReport::new();

    // Step 1: Validate CID format
    if let Err(e) = validate_cid_format(expected_cid) {
        report.errors.push(format!("Invalid CID format: {}", e));
        return Ok(report);
    }

    // Step 2: Verify hash algorithm is supported
    if !is_supported_hash(expected_cid.hash().code()) {
        report.errors.push("Unsupported hash algorithm".to_string());
        return Ok(report);
    }

    // Step 3: Compute actual hash and compare
    let computed_hash = compute_hash(data, expected_cid.hash().code());
    if &computed_hash != expected_cid.hash() {
        report.errors.push(format!(
            "Hash mismatch! Expected: {}, Got: {}",
            expected_cid.hash(),
            computed_hash
        ));
        return Ok(report);
    }

    // Step 4: Verify codec is supported
    if !is_supported_codec(expected_cid.codec()) {
        report.errors.push("Unsupported codec".to_string());
        return Ok(report);
    }

    // Step 5: Validate content structure (for DAG codecs)
    if let Err(e) = validate_content_structure(data, expected_cid.codec()) {
        report.errors.push(format!("Invalid content structure: {}", e));
    }

    report.validated = true;
    Ok(report)
}
```

**Hash Algorithm Whitelist:**

```rust
pub const ALLOWED_HASH_CODES: &[u64] = &[
    0x12,  // SHA-256 (IPFS default)
    0x1e,  // BLAKE3 (modern, fast)
    0xb240, // SHA3-256
];

pub fn is_supported_hash(code: u64) -> bool {
    ALLOWED_HASH_CODES.contains(&code)
}

// Reject deprecated/weak algorithms
pub const REJECTED_HASH_CODES: &[u64] = &[
    0x11,  // SHA-1 (broken)
    0x13,  // SHA-512 (overkill for most uses)
];
```

### 7.2 DHT Attack Mitigation

**Sybil Resistance:**

```rust
pub struct SybilDefense {
    // Require valid peer ID ( cryptographic identity)
    require_valid_peer_id: bool,

    // Rate limit new peer connections
    new_peer_rate_limit: RateLimiter,

    // Require successful handshake
    require_handshake: bool,

    // Peer reputation scoring
    reputation: PeerReputation,
}

impl SybilDefense {
    pub fn should_accept_peer(&self, peer: &PeerInfo) -> bool {
        // Check rate limit
        if !self.new_peer_rate_limit.allow() {
            return false;
        }

        // Verify peer ID is cryptographically valid
        if self.require_valid_peer_id && !peer.id.is_valid() {
            return false;
        }

        // Check reputation score
        if self.reputation.get_score(peer.id) < 0.3 {
            return false;
        }

        true
    }
}
```

**Rate Limiting:**

```rust
use governor::{Quota, RateLimiter};

pub struct DhtRateLimits {
    queries_per_peer: RateLimiter,
    new_peers_per_minute: RateLimiter,
    broadcast_rate: RateLimiter,
}

impl DhtRateLimits {
    pub fn new() -> Self {
        Self {
            queries_per_peer: RateLimiter::direct(
                Quota::per_second(nonzero!(10u32))
            ),
            new_peers_per_minute: RateLimiter::direct(
                Quota::per_minute(nonzero!(50u32))
            ),
            broadcast_rate: RateLimiter::direct(
                Quota::per_second(nonzero!(5u32))
            ),
        }
    }

    pub fn allow_query(&self, peer: &PeerId) -> bool {
        self.queries_per_peer.check_key(peer).is_ok()
    }
}
```

### 7.3 Access Control for Pinning APIs

**RBAC Configuration:**

```rust
pub enum Permission {
    ReadCid,
    WriteCid,
    DeleteCid,
    Admin,
}

pub struct AccessPolicy {
    roles: HashMap<RoleId, HashSet<Permission>>,
    user_roles: HashMap<UserId, RoleId>,
}

impl AccessPolicy {
    pub fn check(&self, user: &UserId, permission: Permission) -> bool {
        if let Some(role) = self.user_roles.get(user) {
            if let Some(permissions) = self.roles.get(role) {
                return permissions.contains(&permission);
            }
        }
        false
    }
}

// Default role configuration
let mut policy = AccessPolicy::new();
policy.add_role("viewer", vec![Permission::ReadCid]);
policy.add_role("editor", vec![Permission::ReadCid, Permission::WriteCid]);
policy.add_role("admin", vec![
    Permission::ReadCid,
    Permission::WriteCid,
    Permission::DeleteCid,
    Permission::Admin,
]);
```

**API Key Management:**

```rust
pub struct ApiKeyManager {
    keys: DashMap<ApiKey, KeyMetadata>,
    rotation_interval: Duration,
}

impl ApiKeyManager {
    pub fn generate_key(&self, owner: UserId, permissions: Vec<Permission>) -> ApiKey {
        let key = ApiKey::new_random();
        let metadata = KeyMetadata {
            owner,
            permissions,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(90),
        };
        self.keys.insert(key.clone(), metadata);
        key
    }

    pub fn validate_key(&self, key: &ApiKey) -> Option<&KeyMetadata> {
        let metadata = self.keys.get(key)?;

        if metadata.expires_at < Utc::now() {
            return None;  // Expired
        }

        Some(metadata.into_inner())
    }

    pub fn rotate_expired(&self) -> usize {
        let now = Utc::now();
        let mut removed = 0;

        self.keys.retain(|_, metadata| {
            if metadata.expires_at < now {
                removed += 1;
                false
            } else {
                true
            }
        });

        removed
    }
}
```

### 7.4 TLS for Peer Connections

**TLS Configuration:**

```rust
use libp2p::noise;
use libp2p::tls;

// Recommended: Noise protocol (default for libp2p)
pub fn create_noise_config() -> Result<noise::Config> {
    let local_key = noise::Keypair::generate_ed25519();
    Ok(noise::Config::new(local_key)?)
}

// Alternative: TLS 1.3
pub fn create_tls_config() -> Result<tls::Config> {
    let cert = tls::Certificate::generate_new()?;
    Ok(tls::Config::new(cert)?)
}

// Transport with encryption
let transport = tcp::tokio::Transport::new(tcp::Config::default())
    .upgrade(libp2p::core::upgrade::Version::V1)
    .authenticate(noise::Config::new(local_key)?)
    .multiplex(yamux::Config::default());
```

**Certificate Management:**

```bash
#!/bin/bash
# generate-peer-cert.sh

# Generate Ed25519 key pair for peer identity
openssl genpkey -algorithm ed25519 -out peer-key.pem

# Extract public key
openssl pkey -in peer-key.pem -pubout -out peer-pubkey.pem

# Generate self-signed certificate (for TLS transport)
openssl req -new -x509 \
    -key peer-key.pem \
    -out peer-cert.pem \
    -days 365 \
    -subj "/CN=peer-$(cat /proc/sys/kernel/random/uuid)"

# Verify certificate
openssl x509 -in peer-cert.pem -text -noout
```

---

## 8. Multi-tenant Deployments

### 8.1 Pinning Service Architecture

**Multi-tenant Service Layout:**

```
┌─────────────────────────────────────────────────────────────────┐
│                     API GATEWAY (Tenant Routing)                 │
│              routes by: API key / subdomain / path               │
└───────────────────────────┬─────────────────────────────────────┘
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
  ┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐
  │  Tenant A   │   │  Tenant B   │   │  Tenant C   │
  │  Namespace  │   │  Namespace  │   │  Namespace  │
  │             │   │             │   │             │
  │ ┌─────────┐ │   │ ┌─────────┐ │   │ ┌─────────┐ │
  │ │ Router  │ │   │ │ Router  │ │   │ │ Router  │ │
  │ └────┬────┘ │   │ └────┬────┘ │   │ └────┬────┘ │
  └──────┼──────┘   └──────┼──────┘   └──────┼──────┘
         │                 │                 │
         └─────────────────┴─────────────────┘
                           │
         ┌─────────────────▼─────────────────┐
         │       Shared Storage Layer         │
         │  ┌─────────┐ ┌─────────┐ ┌──────┐ │
         │  │Prefix-A │ │Prefix-B │ │ ...  │ │
         │  └─────────┘ └─────────┘ └──────┘ │
         └───────────────────────────────────┘
```

**Tenant Isolation Implementation:**

```rust
pub struct TenantContext {
    tenant_id: TenantId,
    namespace: Namespace,
    quotas: ResourceQuotas,
    api_key: ApiKey,
}

pub struct Namespace {
    cid_prefix: String,      // CID namespace prefix
    storage_prefix: String,  // Storage path prefix
    db_schema: String,       // Database schema/table prefix
}

impl TenantContext {
    pub fn isolate_cid(&self, cid: &Cid) -> Result<()> {
        if !cid.to_string().starts_with(&self.namespace.cid_prefix) {
            return Err(anyhow!("CID not in tenant namespace"));
        }
        Ok(())
    }

    pub fn check_quota(&self, usage: &ResourceUsage) -> Result<()> {
        if usage.storage_bytes > self.quotas.max_storage {
            return Err(anyhow!("Storage quota exceeded"));
        }
        if usage.requests_per_minute > self.quotas.max_rpm {
            return Err(anyhow!("Rate limit exceeded"));
        }
        Ok(())
    }
}
```

### 8.2 Tenant Isolation Strategies

**Database-Level Isolation:**

```rust
// Option 1: Separate database per tenant
pub async fn get_tenant_db(tenant_id: &TenantId) -> Result<Db> {
    let db_path = format!("/var/lib/cid-router/tenants/{}/db.sqlite", tenant_id);
    Db::open(&db_path).await
}

// Option 2: Schema per tenant (PostgreSQL)
pub async fn query_tenant_schema(tenant_id: &TenantId, query: &str) -> Result<Vec<Row>> {
    let schema_query = format!(
        "SET search_path TO tenant_{}; {}",
        tenant_id, query
    );
    db.execute(&schema_query).await
}

// Option 3: Row-level isolation (single table)
pub async fn query_tenant_rows(tenant_id: &TenantId) -> Result<Vec<Row>> {
    db.execute(
        "SELECT * FROM routes WHERE tenant_id = $1",
        params![tenant_id]
    ).await
}
```

**Storage Isolation:**

```rust
pub struct TenantStorage {
    tenant_id: TenantId,
    base_path: PathBuf,
}

impl TenantStorage {
    pub fn blob_path(&self, cid: &Cid) -> PathBuf {
        // Tenant isolation via path prefix
        self.base_path
            .join(&self.tenant_id.to_string())
            .join("blobs")
            .join(cid.to_string())
    }

    pub fn validate_access(&self, path: &Path) -> Result<()> {
        // Prevent path traversal attacks
        if !path.starts_with(&self.base_path) {
            return Err(anyhow!("Access denied: path outside tenant boundary"));
        }
        Ok(())
    }
}
```

### 8.3 Resource Quotas and Rate Limiting

**Quota Configuration:**

```rust
#[derive(Debug, Clone)]
pub struct ResourceQuotas {
    // Storage limits
    pub max_storage_bytes: u64,
    pub max_pins: usize,

    // Rate limits
    pub max_requests_per_minute: u64,
    pub max_bandwidth_bytes_per_second: u64,

    // API limits
    pub max_concurrent_requests: usize,
    pub max_request_body_size: usize,
}

impl ResourceQuotas {
    // Free tier
    pub fn free_tier() -> Self {
        Self {
            max_storage_bytes: 1024 * 1024 * 100,      // 100 MB
            max_pins: 100,
            max_requests_per_minute: 60,
            max_bandwidth_bytes_per_second: 1024 * 1024, // 1 MB/s
            max_concurrent_requests: 2,
            max_request_body_size: 1024 * 1024 * 10,   // 10 MB
        }
    }

    // Pro tier
    pub fn pro_tier() -> Self {
        Self {
            max_storage_bytes: 1024 * 1024 * 1024 * 100, // 100 GB
            max_pins: 10000,
            max_requests_per_minute: 600,
            max_bandwidth_bytes_per_second: 1024 * 1024 * 100, // 100 MB/s
            max_concurrent_requests: 10,
            max_request_body_size: 1024 * 1024 * 100,    // 100 MB
        }
    }

    // Enterprise tier
    pub fn enterprise_tier() -> Self {
        Self {
            max_storage_bytes: 1024 * 1024 * 1024 * 1024, // 1 TB
            max_pins: 1000000,
            max_requests_per_minute: 6000,
            max_bandwidth_bytes_per_second: 1024 * 1024 * 1024, // 1 GB/s
            max_concurrent_requests: 100,
            max_request_body_size: 1024 * 1024 * 1024,   // 1 GB
        }
    }
}
```

**Rate Limiter Implementation:**

```rust
use governor::{DirectRateLimiter, Quota, RateLimiter};

pub struct TenantRateLimiter {
    request_limiter: DashMap<TenantId, Arc<DirectRateLimiter>>,
    bandwidth_limiter: DashMap<TenantId, Arc<DirectRateLimiter>>,
}

impl TenantRateLimiter {
    pub fn new(quotas: &ResourceQuotas) -> Self {
        Self {
            request_limiter: DashMap::new(),
            bandwidth_limiter: DashMap::new(),
        }
    }

    pub fn check_request(&self, tenant_id: &TenantId) -> bool {
        let limiter = self.request_limiter
            .entry(tenant_id.clone())
            .or_insert_with(|| {
                Arc::new(RateLimiter::direct(
                    Quota::per_minute(nonzero!(60u32))
                ))
            });

        limiter.check().is_ok()
    }

    pub fn check_bandwidth(&self, tenant_id: &TenantId, bytes: u64) -> bool {
        let limiter = self.bandwidth_limiter
            .entry(tenant_id.clone())
            .or_insert_with(|| {
                Arc::new(RateLimiter::direct(
                    Quota::per_second(nonzero!(1024u32))
                ))
            });

        // Check if request would exceed limit
        limiter.check_n(nonzero!(bytes as u32)).is_ok()
    }
}
```

**Quota Enforcement Middleware:**

```rust
pub async fn quota_middleware(
    State(ctx): State<Arc<AppContext>>,
    auth: AuthHeader,
    request: Request<Body>,
) -> Result<Response, ApiError> {
    // Get tenant context
    let tenant = ctx.get_tenant(&auth.api_key)
        .ok_or_else(|| ApiError::unauthorized("Invalid API key"))?;

    // Check storage quota
    let usage = tenant.get_usage().await?;
    if !tenant.check_quota(&usage).is_ok() {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "Resource quota exceeded"
        ));
    }

    // Check rate limit
    if !ctx.rate_limiter.check_request(&tenant.id) {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded"
        ));
    }

    // Proceed with request
    Ok(next.run(request).await)
}
```

---

## Summary

Production content-addressed data systems require:

1. **High Availability** - Multi-provider redundancy, failover mechanisms, health checks
2. **Scalability** - DHT bucket management, Bitswap parallelism, CRP connection pooling
3. **Performance** - Multi-level caching, batch operations, network optimization
4. **Backup/Recovery** - Pinning policies, disaster recovery, integrity verification
5. **Monitoring** - Kademlia metrics, Bitswap health, storage tracking, Grafana dashboards
6. **Security** - CID validation, Sybil resistance, access control, TLS encryption
7. **Multi-tenancy** - Namespace isolation, resource quotas, rate limiting

---

*This document complements the cid-router deep dives which cover implementation details for the CID Router system.*
