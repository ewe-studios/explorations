# iroh-dns-server Deep Dive

## Overview

`iroh-dns-server` provides a PKARR (Public Key Addressable Resource Record) relay and DNS server for iroh node discovery. It enables nodes to publish and discover each other's connection information via DNS and HTTP endpoints.

**Version:** 0.1.0
**Repository:** https://github.com/n0-computer/iroh-dns-server
**License:** MIT OR Apache-2.0

---

## Architecture and Design Decisions

### PKARR Protocol

PKARR (Public Key Addressable Resource Record) is a protocol for publishing DNS-like records indexed by public keys:

1. **Key-Based Addressing**: Records are identified by ed25519 public keys
2. **Signed Packets**: All records are signed by the corresponding private key
3. **DNS Compatibility**: Uses standard DNS packet format
4. **DHT Compatible**: Can be distributed over DHTs like Mainline

### Server Components

The server provides two main services:

1. **PKARR Relay**: HTTP endpoint for publishing and fetching signed packets
2. **DNS Server**: Standard DNS server for resolving iroh node IDs

### Design Decisions

1. **Rate Limiting**: Governor-based rate limiting to prevent abuse
2. **Caching**: LRU cache for frequently accessed records
3. **Persistent Storage**: redb database for record persistence
4. **TLS Support**: Optional TLS with auto-certificate via ACME
5. **CORS**: Configurable CORS for web-based clients

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    iroh-dns-server                       │
├─────────────────────────────────────────────────────────┤
│  HTTP Server (Axum)            DNS Server (Hickory)     │
│  ┌────────────────────┐        ┌────────────────────┐   │
│  │  POST /pkarr       │        │  DNS Query Handler │   │
│  │  GET  /pkarr/:key  │        │  A/AAAA Records    │   │
│  │  GET  /stats       │        │  HTTPS Records     │   │
│  └────────────────────┘        └────────────────────┘   │
├─────────────────────────────────────────────────────────┤
│                    Application Layer                     │
│  ┌───────────────┐  ┌───────────────┐  ┌─────────────┐  │
│  │ Rate Limiter  │  │ LRU Cache     │  │ Metrics     │  │
│  └───────────────┘  └───────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────────┤
│                    Storage Layer                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │              redb Database                        │  │
│  │  - Signed packets by public key                   │  │
│  │  - Timestamp index                                │  │
│  │  - Metrics data                                   │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## Key APIs and Data Structures

### Server Configuration

```rust
/// Server configuration
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    /// HTTP server address
    pub http_addr: SocketAddr,

    /// DNS server address
    pub dns_addr: SocketAddr,

    /// TLS configuration
    pub tls: Option<TlsConfig>,

    /// Database path
    pub database_path: PathBuf,

    /// Rate limiting
    pub rate_limit: RateLimitConfig,

    /// Cache size
    pub cache_size: usize,
}

/// TLS configuration
#[derive(Clone, Debug, serde::Deserialize)]
pub struct TlsConfig {
    /// ACME configuration for auto-certificates
    pub acme: Option<AcmeConfig>,

    /// Manual certificate paths
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
}
```

### PKARR Packet

```rust
/// Signed packet for PKARR
pub struct SignedPacket {
    /// Public key (node ID)
    public_key: PublicKey,

    /// Signature
    signature: Signature,

    /// DNS packet data
    packet: Vec<u8>,
}

impl SignedPacket {
    /// Create new signed packet
    pub fn new(
        secret_key: &SecretKey,
        records: Vec<DnsRecord>,
    ) -> Result<Self> {
        // Create DNS packet
        let packet = encode_dns_packet(records)?;

        // Sign the packet
        let signature = secret_key.sign(&packet);

        Ok(Self {
            public_key: secret_key.public(),
            signature,
            packet,
        })
    }

    /// Verify signature
    pub fn verify(&self) -> bool {
        self.public_key.verify(&self.packet, &self.signature).is_ok()
    }

    /// Encode to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        // Format: [public_key (32)][signature (64)][packet (variable)]
        let mut buf = Vec::new();
        buf.extend_from_slice(self.public_key.as_bytes());
        buf.extend_from_slice(self.signature.as_bytes());
        buf.extend_from_slice(&self.packet);
        buf
    }

    /// Decode from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Parse and validate
    }
}
```

### Store Interface

```rust
/// Packet storage trait
pub trait Store: Clone + Send + Sync + 'static {
    /// Store a signed packet
    fn put(&self, packet: SignedPacket) -> Result<()>;

    /// Get packet by public key
    fn get(&self, key: &PublicKey) -> Result<Option<SignedPacket>>;

    /// Delete packet by public key
    fn delete(&self, key: &PublicKey) -> Result<()>;

    /// List all stored keys
    fn list(&self) -> Result<Vec<PublicKey>>;
}

/// redb-based implementation
pub struct RedbStore {
    db: redb::Database,
}

impl Store for RedbStore {
    fn put(&self, packet: SignedPacket) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(PACKETS)?;
            let key = packet.public_key.as_bytes();
            let value = packet.to_bytes();
            table.insert(key, value.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    fn get(&self, key: &PublicKey) -> Result<Option<SignedPacket>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PACKETS)?;
        match table.get(key.as_bytes())? {
            Some(value) => {
                let packet = SignedPacket::from_bytes(value.value())?;
                Ok(Some(packet))
            }
            None => Ok(None),
        }
    }
}
```

### HTTP Handlers

```rust
/// HTTP router state
#[derive(Clone)]
pub struct AppState {
    store: Arc<dyn Store>,
    cache: Arc<LruCache<PublicKey, SignedPacket>>,
    rate_limiter: Arc<RateLimiter>,
    metrics: Arc<Metrics>,
}

/// Publish PKARR packet
async fn publish_pkarr(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    // Check rate limit
    if !state.rate_limiter.check() {
        return Ok(StatusCode::TOO_MANY_REQUESTS);
    }

    // Parse and validate packet
    let packet = SignedPacket::from_bytes(&body)
        .map_err(|_| AppError::InvalidPacket)?;

    // Verify signature
    if !packet.verify() {
        return Ok(StatusCode::UNAUTHORIZED);
    }

    // Store packet
    state.store.put(packet.clone())?;

    // Update cache
    state.cache.put(packet.public_key, packet);

    // Update metrics
    state.metrics.publishes.inc();

    Ok(StatusCode::OK)
}

/// Fetch PKARR packet
async fn fetch_pkarr(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Response, AppError> {
    // Parse key
    let key = parse_public_key(&key)?;

    // Check cache first
    if let Some(packet) = state.cache.get(&key) {
        state.metrics.cache_hits.inc();
        return Ok(packet.to_bytes().into_response());
    }

    // Fetch from store
    state.metrics.cache_misses.inc();
    match state.store.get(&key)? {
        Some(packet) => {
            // Update cache
            state.cache.put(packet.public_key, packet.clone());
            Ok(packet.to_bytes().into_response())
        }
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}
```

### DNS Handler

```rust
use hickory_server::authority::{Catalog, MessageRequest, MessageResponse};
use hickory_server::server::RequestHandler;

/// DNS request handler
pub struct DnsHandler {
    catalog: Catalog,
}

#[async_trait::async_trait]
impl RequestHandler for DnsHandler {
    async fn handle_request(
        &self,
        request: &MessageRequest,
        response_handle: ResponseHandle,
    ) -> ResponseCode {
        // Build response
        let mut response = MessageResponse::new(
            request.id(),
            request.op_code(),
            ResponseCode::NoError,
        );

        // Handle based on query type
        for query in request.queries() {
            match query.query_type() {
                RecordType::A => {
                    // Handle IPv4 address query
                    handle_a_query(query, &mut response).await?;
                }
                RecordType::AAAA => {
                    // Handle IPv6 address query
                    handle_aaaa_query(query, &mut response).await?;
                }
                RecordType::HTTPS => {
                    // Handle HTTPS record query (for relay info)
                    handle_https_query(query, &mut response).await?;
                }
                _ => {
                    response.set_response_code(ResponseCode::NXDomain);
                }
            }
        }

        // Send response
        response_handle.send(response).await?;
        ResponseCode::NoError
    }
}

/// Handle A/AAAA query for node ID
async fn handle_a_query(
    query: &Query,
    response: &mut MessageResponse,
) -> Result<(), AppError> {
    // Node ID is encoded in subdomain
    // e.g., <node-id>.node.example.com
    let node_id = extract_node_id(query.name())?;

    // Look up node info
    let packet = store.get(&node_id)?;

    if let Some(packet) = packet {
        // Extract addresses from DNS records
        let addresses = extract_addresses(&packet)?;

        // Add A records to response
        for addr in addresses.ipv4 {
            response.add_answer(record::A::new(
                query.name(),
                addr,
                300, // TTL
            ));
        }
    }

    Ok(())
}
```

---

## Protocol Details

### PKARR Wire Format

```
┌─────────────────────────────────────────────────────────┐
│                    Signed Packet                        │
├─────────────────────────────────────────────────────────┤
│  Public Key (32 bytes)     │ ed25519 public key         │
├─────────────────────────────────────────────────────────┤
│  Signature (64 bytes)      │ ed25519 signature          │
├─────────────────────────────────────────────────────────┤
│  DNS Packet (variable)     │ Standard DNS wire format   │
└─────────────────────────────────────────────────────────┘
```

### DNS Record Format

Published records follow standard DNS format:

```rust
// Example DNS records for a node
let records = vec![
    // A record (IPv4)
    DnsRecord::A {
        name: "node.example.com",
        address: Ipv4Addr::new(192, 168, 1, 1),
        ttl: 300,
    },

    // AAAA record (IPv6)
    DnsRecord::AAAA {
        name: "node.example.com",
        address: Ipv6Addr::LOCALHOST,
        ttl: 300,
    },

    // HTTPS record (relay info)
    DnsRecord::HTTPS {
        name: "node.example.com",
        priority: 1,
        target: "relay.example.com",
        params: vec!["alpn=h3"],
        ttl: 300,
    },
];
```

### HTTP API

```
POST /pkarr
Content-Type: application/octet-stream
[Body: SignedPacket bytes]

Response: 200 OK (success)
          400 Bad Request (invalid packet)
          401 Unauthorized (bad signature)
          429 Too Many Requests (rate limited)

GET /pkarr/:key
Response: 200 OK [Body: SignedPacket bytes]
          404 Not Found

GET /stats
Response: 200 OK [Body: JSON metrics]
```

---

## Integration with Main Iroh Endpoint

### Node Publishing

```rust
use iroh::{Endpoint, NodeAddr};

// Publish node info to DNS server
async fn publish_node_info(
    endpoint: &Endpoint,
    dns_url: &str,
) -> Result<()> {
    let node_id = endpoint.node_id();
    let relay_url = endpoint.relay_url().await?;
    let direct_addresses = endpoint.direct_addresses().await?;

    // Create DNS records
    let records = vec![
        // Add direct addresses
        for addr in direct_addresses {
            match addr {
                SocketAddr::V4(v4) => DnsRecord::A {
                    name: format!("{}.node.iroh.local", node_id.fmt_short()),
                    address: v4.ip(),
                    ttl: 300,
                },
                SocketAddr::V6(v6) => DnsRecord::AAAA {
                    name: format!("{}.node.iroh.local", node_id.fmt_short()),
                    address: v6.ip(),
                    ttl: 300,
                },
            }
        },

        // Add relay info
        if let Some(relay) = relay_url {
            DnsRecord::HTTPS {
                name: format!("{}.node.iroh.local", node_id.fmt_short()),
                priority: 1,
                target: relay.host().to_string(),
                params: vec!["alpn=h3"],
                ttl: 300,
            }
        },
    ];

    // Sign and publish
    let packet = SignedPacket::new(endpoint.secret_key(), records)?;

    let client = reqwest::Client::new();
    client.post(&format!("{}/pkarr", dns_url))
        .body(packet.to_bytes())
        .send()
        .await?;

    Ok(())
}
```

### Node Discovery

```rust
// Discover node via DNS
async fn discover_node(
    node_id: PublicKey,
    dns_domain: &str,
) -> Result<NodeAddr> {
    let resolver = hickory_resolver::TokioResolver::tokio_from_system_conf()?;

    // Query for A/AAAA records
    let name = format!("{}.node.{}", node_id.fmt_short(), dns_domain);

    let mut addr = NodeAddr::new(node_id);

    // Get IPv4 addresses
    if let Ok(response) = resolver.lookup_ip(&name).await {
        for record in response.iter() {
            addr = addr.with_direct_addr(SocketAddr::new(record, 0));
        }
    }

    // Get HTTPS record for relay info
    if let Ok(response) = resolver.lookup(name, RecordType::HTTPS).await {
        for record in response.iter() {
            if let RData::HTTPS(https) = record {
                addr = addr.with_relay_url(https.target().to_string());
            }
        }
    }

    Ok(addr)
}
```

---

## Production Usage Patterns

### Running the Server

```rust
use iroh_dns_server::{Config, Server};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config: Config = toml::from_str(&std::fs::read_to_string("config.toml")?)?;

    // Create and run server
    let server = Server::new(config).await?;
    server.run().await?;

    Ok(())
}
```

### Configuration File

```toml
# config.toml

# HTTP server
http_addr = "0.0.0.0:8080"

# DNS server
dns_addr = "0.0.0.0:53"

# Database
database_path = "/var/lib/iroh-dns/db"

# Cache
cache_size = 10000

# Rate limiting
[rate_limit]
requests_per_second = 10
burst_size = 100

# TLS (optional)
[tls]
acme_domain = "dns.example.com"
acme_email = "admin@example.com"
```

### Metrics Collection

```rust
// Prometheus metrics endpoint
async fn metrics_handler(
    State(metrics): State<Arc<Metrics>>,
) -> impl IntoResponse {
    let output = metrics.render_prometheus();
    (
        [(CONTENT_TYPE, "text/plain; version=0.0.4")],
        output,
    )
}

// Example metrics:
// iroh_dns_publishes_total 1234
// iroh_dns_fetches_total 5678
// iroh_dns_cache_hits_total 4000
// iroh_dns_cache_misses_total 1678
// iroh_dns_stored_keys 890
```

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| axum | 0.7.4 | HTTP server framework |
| hickory-server | 0.24.0 | DNS server |
| hickory-proto | 0.24.0 | DNS protocol |
| hickory-resolver | 0.24.0 | DNS resolver |
| redb | 2.0.0 | Embedded database |
| governor | 0.6.3 | Rate limiting |
| lru | 0.12.3 | LRU cache |
| tokio-rustls-acme | git | ACME TLS certificates |
| pkarr | 1.1.2 | PKARR protocol |

### Notable Rust Patterns

1. **Trait Objects for Storage**: `dyn Store` for pluggable backends
2. **Arc-based Sharing**: Thread-safe sharing of server state
3. **Type-safe Configuration**: serde for config parsing
4. **Async-first Design**: All operations are async

### Performance Considerations

1. **LRU Caching**: Reduces database lookups for hot keys
2. **Rate Limiting**: Governor token bucket for API protection
3. **Connection Pooling**: Reused HTTP client connections
4. **Batched Writes**: Grouped database operations

### Potential Enhancements

1. **DHT Integration**: Publish to Mainline DHT
2. **Replication**: Multi-node replication for HA
3. **TTL Enforcement**: Automatic record expiration
4. **Metrics Export**: Additional export formats

---

## Summary

`iroh-dns-server` provides:

- **PKARR Relay**: HTTP endpoint for node info publication
- **DNS Resolution**: Standard DNS for node discovery
- **Rate Limiting**: Protection against abuse
- **Persistent Storage**: redb-based record storage
- **Auto-TLS**: ACME-based certificate management

The server is a critical component of iroh's node discovery infrastructure.
