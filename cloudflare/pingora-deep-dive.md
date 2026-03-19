---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/Others/pingora
repository: https://github.com/cloudflare/pingora
revised_at: 2026-03-19
---

# Pingora Deep Dive: Production HTTP Proxy Framework

## Overview

Pingora is Cloudflare's Rust framework for building fast, reliable, and programmable network proxies. It powers Cloudflare's edge proxy infrastructure, handling millions of requests per second with support for HTTP/1.x, HTTP/2, and HTTP/3 (via quiche integration).

## Workspace Architecture

### Crate Structure

```
pingora/
├── pingora-core/          # Core server, connection management, TLS abstraction
├── pingora-proxy/         # HTTP proxy implementation (upstream connections)
├── pingora-cache/         # Response caching with configurable policies
├── pingora-load-balancing/# Load balancing with health checks
├── pingora-pool/          # Connection pooling and multiplexing
├── pingora-limits/        # Rate limiting at multiple levels
├── pingora-lru/           # LRU cache implementation
├── pingora-header-serde/  # Header serialization/deserialization
├── pingora-http/          # HTTP utilities and protocol handling
├── pingora-runtime/       # Tokio runtime configuration
├── pingora-timeout/       # Timeout handling utilities
├── pingora-boringssl/     # BoringSSL TLS backend
├── pingora-openssl/       # OpenSSL TLS backend
├── pingora-rustls/        # Rustls TLS backend
├── pingora-ketama/        # Consistent hashing for load balancing
├── pingora-memory-cache/  # In-memory cache implementation
├── tinyufo/               # (Purpose TBD)
└── pingora/               # Meta-crate re-exporting all components
```

### Workspace Dependencies

```toml
[workspace.dependencies]
tokio = "1"
async-trait = "0.1.42"
httparse = "1"
bytes = "1.0"
http = "1.0.0"
log = "0.4"
h2 = ">=0.4.6"
once_cell = "1"
lru = "0"
ahash = ">=0.8.9"
```

## Core Server Architecture

### Server Struct and Lifecycle

The `Server` struct in `pingora-core/src/server/mod.rs` is the central orchestration point:

```rust
pub struct Server {
    services: Vec<Box<dyn Service>>,
    #[cfg(unix)]
    listen_fds: Option<ListenFds>,
    shutdown_watch: watch::Sender<bool>,
    shutdown_recv: ShutdownWatch,
    pub configuration: Arc<ServerConf>,
    pub options: Option<Opt>,
    #[cfg(feature = "sentry")]
    pub sentry: Option<ClientOptions>,
}
```

**Key Design Decisions:**

1. **Service-based Architecture**: Multiple independent services run within a single server process
2. **Watch Channel for Shutdown**: Uses `tokio::sync::watch` for broadcasting shutdown signals
3. **Optional Sentry Integration**: Error reporting feature-gated for release builds only

### Graceful Shutdown Constants

```rust
const EXIT_TIMEOUT: u64 = 60 * 5;  // 5 minutes for existing sessions
const CLOSE_TIMEOUT: u64 = 5;       // 5 seconds for new service readiness
```

### Signal Handling

```rust
async fn main_loop(&self) -> ShutdownType {
    let mut graceful_upgrade_signal = unix::signal(unix::SignalKind::quit()).unwrap();
    let mut graceful_terminate_signal = unix::signal(unix::SignalKind::terminate()).unwrap();
    let mut fast_shutdown_signal = unix::signal(unix::SignalKind::interrupt()).unwrap();

    tokio::select! {
        _ = fast_shutdown_signal.recv() => {
            info!("SIGINT received, exiting");
            ShutdownType::Quick
        },
        _ = graceful_terminate_signal.recv() => {
            info!("SIGTERM received, gracefully exiting");
            ShutdownType::Graceful
        }
        _ = graceful_upgrade_signal.recv() => {
            info!("SIGQUIT received, sending socks and gracefully exiting");
            // Zero downtime upgrade: send listening sockets to new process
        },
    }
}
```

**Three Shutdown Modes:**

| Signal | Type | Behavior |
|--------|------|----------|
| SIGQUIT | Graceful Upgrade | Send listening sockets to new process, then graceful shutdown |
| SIGTERM | Graceful | Wait 5 minutes for sessions to complete |
| SIGINT | Quick | Immediate exit |

### Zero Downtime Upgrade via Socket Passing

```rust
fn load_fds(&mut self, upgrade: bool) -> Result<(), nix::Error> {
    let mut fds = Fds::new();
    if upgrade {
        debug!("Trying to receive socks");
        fds.get_from_sock(self.configuration.as_ref().upgrade_sock.as_str())?
    }
    self.listen_fds = Some(Arc::new(Mutex::new(fds)));
    Ok(())
}
```

The upgrade mechanism:
1. Old server receives SIGQUIT
2. Old server sends listening file descriptors via Unix socket to `upgrade_sock` path
3. New server starts and receives the file descriptors
4. New server accepts connections on existing sockets
5. Old server gracefully drains existing connections

### Runtime Configuration

```rust
fn create_runtime(name: &str, threads: usize, work_steal: bool) -> Runtime {
    if work_steal {
        Runtime::new_steal(threads, name)
    } else {
        Runtime::new_no_steal(threads, name)
    }
}
```

**Work Stealing vs Non-Work Stealing:**

| Mode | Use Case | Characteristics |
|------|----------|-----------------|
| Work Stealing | General purpose | Tasks can be stolen between threads, better load balancing |
| Non-Work Stealing | Deterministic workloads | Tasks stay on their thread, better cache locality |

## HTTP Protocol Implementation

### HttpTask Response Events

```rust
pub enum HttpTask {
    Header(Box<pingora_http::ResponseHeader>, bool),  // Response header + end flag
    Body(Option<bytes::Bytes>, bool),                  // Body chunk + end flag
    Trailer(Option<Box<http::HeaderMap>>),             // Trailers
    Done,                                               // Response complete
    Failed(pingora_error::BError),                      // Error occurred
}
```

This enum drives the streaming response pattern:

```rust
impl HttpTask {
    pub fn is_end(&self) -> bool {
        match self {
            HttpTask::Header(_, end) => *end,
            HttpTask::Body(_, end) => *end,
            HttpTask::Trailer(_) => true,
            HttpTask::Done => true,
            HttpTask::Failed(_) => true,
        }
    }
}
```

### Server Name Constant

```rust
pub const SERVER_NAME: &[u8; 7] = b"Pingora";
```

Used for the `Server` header in HTTP responses.

## TLS Backend Abstraction

Pingora supports multiple TLS backends through feature flags:

```toml
[dependencies]
# Feature-gated TLS backends
pingora-boringssl = { path = "../pingora-boringssl", optional = true }
pingora-openssl = { path = "../pingora-openssl", optional = true }
pingora-rustls = { path = "../pingora-rustls", optional = true }
```

**Backend Characteristics:**

| Backend | Performance | FIPS | Use Case |
|---------|-------------|------|----------|
| BoringSSL | Highest | Yes (with FIPS module) | Production at scale |
| OpenSSL | High | With FIPS object module | Legacy compatibility |
| Rustls | Good | No (pure Rust, no FIPS validation) | Security-critical, auditability |

## Connection Pooling (pingora-pool)

The connection pool manages upstream connections with:

1. **Idle Connection Reuse**: Keep-alive connections are reused
2. **Configurable Limits**: Max connections per upstream
3. **Health Checking**: Unhealthy connections are removed
4. **Exponential Backoff**: Failed connection attempts back off

## Load Balancing (pingora-load-balancing)

### Consistent Hashing with Ketama

The `pingora-ketama` crate implements consistent hashing for deterministic request routing:

- Virtual nodes per backend for even distribution
- Minimal redistribution when backends change
- O(log N) lookup time

### Health Check Integration

```rust
// Conceptual example
let load_balancer = LoadBalancer::new()
    .with_health_checks(HealthCheckConfig {
        interval: Duration::from_secs(10),
        timeout: Duration::from_secs(5),
        unhealthy_threshold: 3,
    });
```

## Rate Limiting (pingora-limits)

Multi-level rate limiting:

1. **Global Limits**: Across all workers
2. **Per-IP Limits**: Rate limit by client IP
3. **Per-Route Limits**: Protect specific endpoints
4. **Burst Allowance**: Short bursts above limit

## Cache Implementation (pingora-cache)

### Cache Key Generation

Cache keys are generated from:
- Request URI
- Host header
- Vary headers
- Cookie configuration (strip or include)

### Cache Policies

```rust
// Conceptual cache policy configuration
CachePolicy {
    respect_origin_headers: true,  // Honor Cache-Control from origin
    default_ttl: Duration::from_secs(3600),
    max_ttl: Duration::from_secs(86400),
    stale_while_revalidate: Duration::from_secs(600),
}
```

## Memory Management

### LRU Cache (pingora-lru)

```rust
use pingora_lru::LruCache;

let mut cache = LruCache::new(10000);  // 10k entries
cache.insert(key, value);
```

Features:
- O(1) insert and lookup
- Automatic eviction of least-recently-used entries
- Configurable capacity

### Buffer Management

All HTTP body handling uses `bytes::Bytes` for:
- Zero-copy cloning
- Efficient subslicing
- Reference-counted memory sharing

## Error Handling

### Error Types

```rust
// Using thiserror for error definitions
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("Upstream connection failed: {0}")]
    UpstreamConnection(#[from] io::Error),

    #[error("TLS handshake failed: {0}")]
    TlsHandshake(String),

    #[error("Cache miss for key: {0}")]
    CacheMiss(String),
}
```

### Error Propagation Pattern

```rust
pub async fn handle_request(req: &Request) -> Result<Response> {
    let upstream = get_upstream()?;  // ? for early return
    let response = upstream.send(req).await?;
    Ok(response)
}
```

## Async Runtime Integration

### Tokio Runtime Configuration

```rust
// In pingora-runtime
Runtime::new_steal(threads, name)  // Multi-threaded with work stealing
```

Default configuration:
- Thread pool sized to CPU count
- Work stealing enabled by default
- Configurable via `ServerConf`

### Async Trait Pattern

```rust
use async_trait::async_trait;

#[async_trait]
pub trait ProxyRequestHandler {
    async fn request_body(&mut self, body: &mut RequestBody) -> Result<()>;
    async fn response_body(&mut self, body: &mut ResponseBody) -> Result<()>;
}
```

## Logging and Observability

### Tracing Integration

```rust
use log::{info, debug, error, warn};

info!("Server starting");
debug!("Configuration: {:#?}", config);
error!("Connection failed: {:?}", error);
```

### Sentry Error Reporting

```rust
#[cfg(all(not(debug_assertions), feature = "sentry"))]
let _guard = self.sentry.as_ref().map(|opts| sentry::init(opts.clone()));

// Error capture
#[cfg(all(not(debug_assertions), feature = "sentry"))]
sentry::capture_error(&e);
```

## Performance Optimizations

### 1. Connection Reuse

- Keep-alive connections pooled per upstream
- Configurable idle timeout
- Automatic reconnection on stale connections

### 2. Header Handling

- Header serialization via `pingora-header-serde`
- Efficient header modification without reallocation
- Header map reuse across requests

### 3. Zero-Copy Body Handling

```rust
// Body chunks are bytes::Bytes (reference counted)
Body(Option<bytes::Bytes>, bool)
```

### 4. Work Stealing Scheduler

- Better load balancing across threads
- Reduced tail latency for long-running requests

## Building and Deployment

### Feature Flags

```toml
[features]
default = ["boringssl"]
boringssl = ["pingora-boringssl"]
openssl = ["pingora-openssl"]
rustls = ["pingora-rustls"]
cache = ["pingora-cache"]
```

### Release Profile Optimization

```toml
[profile.release]
lto = true
codegen-units = 1
```

## Testing Strategy

### Unit Tests

- Per-module `mod tests` blocks
- Mock upstream servers
- Simulated network failures

### Integration Tests

- Full proxy round-trip tests
- TLS handshake verification
- Cache behavior validation

### Benchmarking

```toml
[profile.bench]
debug = true  # Include debug info for profiling
```

## Common Patterns

### Service Implementation

```rust
use pingora_core::server::Server;
use pingora_core::services::Service;

struct MyProxyService;

#[async_trait]
impl Service for MyProxyService {
    async fn start_service(&mut self, shutdown: ShutdownWatch) {
        // Service implementation
    }
}

fn main() {
    let mut server = Server::new(None).unwrap();
    server.add_service(MyProxyService);
    server.run_forever();
}
```

### Proxy Request Handler

```rust
#[async_trait]
impl ProxyRequestHandler for MyHandler {
    async fn request_body(&mut self, body: &mut RequestBody) -> Result<()> {
        // Modify request body
        Ok(())
    }

    async fn response_body(&mut self, body: &mut ResponseBody) -> Result<()> {
        // Modify response body
        Ok(())
    }
}
```

## Security Considerations

### TLS Configuration

- Minimum TLS 1.2 enforced
- Strong cipher suites only
- OCSP stapling enabled
- Session ticket rotation

### Rate Limiting

- Request rate limiting per IP
- Connection rate limiting
- Request size limits

### Header Sanitization

- Hop-by-hop header removal
- Request normalization
- Response header filtering

## References

- [Pingora GitHub](https://github.com/cloudflare/pingora)
- [Pingora Documentation](https://docs.rs/pingora)
- [Cloudflare Blog: Pingora Announcement](https://blog.cloudflare.com/pingora/)
