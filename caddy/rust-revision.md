# reproducing Caddy in Rust - Comprehensive Guide

## Executive Summary

This document provides a detailed roadmap for reproducing Caddy's functionality in Rust at production level. Caddy is a sophisticated web server platform with automatic HTTPS, modular architecture, and robust certificate management. Reproducing it in Rust requires careful attention to several complex subsystems.

**Key Challenges:**
1. ACME protocol implementation (RFC 8555)
2. TLS handshake integration
3. Distributed coordination for clusters
4. Graceful connection handling
5. OCSP stapling infrastructure

**Rust Advantages:**
- Memory safety without garbage collection
- Zero-cost abstractions for performance
- Strong type system for correctness
- Excellent async runtime ecosystem (tokio, async-std)
- Growing TLS/ cryptography ecosystem (rustls, tokio-rustls)

## Architecture Overview

### Target System Components

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Rust Caddy Equivalent                            │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    Application Layer                         │    │
│  │  - CLI (clap)                                                │    │
│  │  - Configuration parser (serde)                              │    │
│  │  - Module system (trait-based)                               │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                               │                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                     Core Server                              │    │
│  │  - Listener management (tokio::net)                          │    │
│  │  - Connection handling                                       │    │
│  │  - Graceful shutdown                                         │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                               │                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                   HTTP Stack                                 │    │
│  │  - HTTP/1.1, HTTP/2, HTTP/3 (hyper, h3)                      │    │
│  │  - Middleware system                                         │    │
│  │  - Routing                                                   │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                               │                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                   TLS Management                             │    │
│  │  - rustls integration                                        │    │
│  │  - Certificate cache                                         │    │
│  │  - OCSP stapling                                             │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                               │                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                 ACME Client                                  │    │
│  │  - Protocol implementation                                   │    │
│  │  - Challenge solvers                                         │    │
│  │  - Certificate renewal                                       │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                               │                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                  Storage Backend                             │    │
│  │  - Trait-based abstraction                                   │    │
│  │  - File system implementation                                │    │
│  │  - Distributed storage adapters                              │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

### Recommended Crate Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["codec"] }

# HTTP stack
hyper = { version = "1", features = ["full", "http1", "http2"] }
hyper-util = { version = "0.1", features = ["full"] }
http = "1"
http-body-util = "0.1"

# HTTP/3 (optional but recommended)
h3 = "0.0.6"
h3-quinn = "0.0.7"
quinn = "0.11"

# TLS
rustls = "0.23"
tokio-rustls = "0.26"
rustls-pemfile = "2"
rustls-pki-types = "1"

# ACME client (or build your own)
acme2 = "3"  # High-level ACME client
# OR
rcgen = "0.13"  # For certificate generation
x509-parser = "0.16"  # For certificate parsing

# Cryptography
ring = "0.17"  # Low-level crypto
rustls-webpki = "0.102"  # WebPKI verification

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"  # Optional config format
toml = "0.8"  # Optional config format

# Configuration
config = "0.14"  # Config file parsing

# CLI
clap = { version = "4", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
thiserror = "1"
anyhow = "1"

# Utilities
bytes = "1"
pin-project-lite = "0.2"
futures = "0.3"
async-trait = "0.1"

# Storage backends
tokio-fs = "0.1"  # Async file system
redis = { version = "0.25", features = ["tokio-comp"] }  # Optional
# Add database clients as needed

# Time
chrono = { version = "0.4", features = ["serde"] }
```

## Core Systems Implementation

### 1. Module System (Trait-Based)

Rust doesn't have dynamic module loading like Go, but we can use trait objects:

```rust
use async_trait::async_trait;
use serde::Deserialize;
use std::any::Any;
use std::sync::Arc;

/// Base trait for all modules
#[async_trait]
pub trait Module: Send + Sync {
    /// Module identifier (e.g., "http.handlers.file_server")
    fn id(&self) -> &'static str;

    /// Cast to Any for downcasting
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Modules that need initialization
#[async_trait]
pub trait Provisioner: Module {
    type Context: Send + Sync;

    async fn provision(&mut self, ctx: &Self::Context) -> Result<(), ModuleError>;
}

/// Modules that can validate their configuration
#[async_trait]
pub trait Validator: Module {
    async fn validate(&self) -> Result<(), ModuleError>;
}

/// Modules that need cleanup
#[async_trait]
pub trait CleanerUpper: Module {
    async fn cleanup(&mut self) -> Result<(), ModuleError>;
}

/// Module registry
pub struct ModuleRegistry {
    modules: std::collections::HashMap<String, ModuleFactory>,
}

type ModuleFactory = Arc<dyn Fn() -> Box<dyn Module> + Send + Sync>;

impl ModuleRegistry {
    pub fn register<M: Module + Default + 'static>(&mut self, id: &'static str) {
        self.modules.insert(
            id.to_string(),
            Arc::new(|| Box::new(M::default())),
        );
    }

    pub fn create(&self, id: &str) -> Option<Box<dyn Module>> {
        self.modules.get(id).map(|factory| factory())
    }
}

/// Example: HTTP handler module
#[async_trait]
pub trait HttpHandler: Module {
    async fn serve_http(
        &self,
        req: http::Request<hyper::Body>,
        next: Next<'_>,
    ) -> Result<http::Response<hyper::Body>, ModuleError>;
}
```

### 2. Configuration System

```rust
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub apps: HashMap<String, AppConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AppConfig {
    Http(HttpConfig),
    Tls(TlsConfig),
    Pki(PkiConfig),
    // ... other apps
}

#[derive(Debug, Deserialize)]
pub struct HttpConfig {
    pub servers: HashMap<String, ServerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub listen: Vec<String>,
    pub routes: Vec<RouteConfig>,
}

#[derive(Debug, Deserialize)]
pub struct RouteConfig {
    #[serde(default)]
    pub match_: RouteMatch,

    #[serde(default)]
    pub handle: Vec<HandlerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct HandlerConfig {
    pub handler: String,  // Module name

    #[serde(flatten)]
    pub config: serde_json::Value,  // Module-specific config
}

/// Configuration loader
pub struct ConfigLoader {
    registry: Arc<ModuleRegistry>,
}

impl ConfigLoader {
    pub async fn load(&self, path: &str) -> Result<Config, ConfigError> {
        let content = tokio::fs::read_to_string(path).await?;

        // Parse JSON config
        let config: Config = serde_json::from_str(&content)?;

        // Validate and instantiate modules
        self.validate_modules(&config).await?;

        Ok(config)
    }

    async fn validate_modules(&self, config: &Config) -> Result<(), ConfigError> {
        for (app_name, app_config) in &config.apps {
            // Load module for this app
            let mut module = self.registry
                .create(app_name)
                .ok_or_else(|| ConfigError::UnknownModule(app_name.clone()))?;

            // If it's a validator, validate
            if let Some(validator) = module.as_any().downcast_ref::<dyn Validator>() {
                validator.validate().await?;
            }
        }
        Ok(())
    }
}
```

### 3. Listener Management

```rust
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpSocket};
use tokio::sync::RwLock;

/// Network address representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkAddress {
    pub network: String,  // "tcp", "udp", "unix"
    pub host: String,
    pub port_range: Option<(u16, u16)>,
}

impl NetworkAddress {
    pub fn parse(addr: &str) -> Result<Self, AddrParseError> {
        // Parse "tcp/:8080" or "/path/to/socket"
        // Similar to Caddy's ParseNetworkAddress
    }

    pub async fn listen(&self, config: &ListenConfig) -> Result<Listener, ListenerError> {
        match self.network.as_str() {
            "tcp" => self.listen_tcp(config).await,
            "unix" => self.listen_unix().await,
            "udp" => self.listen_udp().await,
            _ => Err(ListenerError::UnsupportedNetwork(self.network.clone())),
        }
    }

    async fn listen_tcp(&self, config: &ListenConfig) -> Result<Listener, ListenerError> {
        // SO_REUSEPORT for graceful reloads
        let socket = TcpSocket::new_v4()?;

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = socket.as_raw_fd();
            unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_REUSEPORT,
                    &1 as *const _ as *const _,
                    std::mem::size_of::<i32>() as libc::socklen_t,
                );
            }
        }

        socket.bind(self.to_socket_addr()?)?;
        let listener = socket.listen(config.backlog.unwrap_or(1024))?;

        Ok(Listener::Tcp(TcpListener::from_std(listener)?))
    }
}

/// Listener manager for graceful reloads
pub struct ListenerManager {
    listeners: Arc<RwLock<HashMap<String, Arc<Listener>>>>,
}

impl ListenerManager {
    pub async fn get_or_create(
        &self,
        addr: &NetworkAddress,
        config: &ListenConfig,
    ) -> Result<Arc<Listener>, ListenerError> {
        let key = addr.to_string();

        // Check if listener exists
        {
            let listeners = self.listeners.read().await;
            if let Some(listener) = listeners.get(&key) {
                return Ok(listener.clone());
            }
        }

        // Create new listener
        let mut listeners = self.listeners.write().await;

        // Double-check after acquiring write lock
        if let Some(listener) = listeners.get(&key) {
            return Ok(listener.clone());
        }

        let listener = Arc::new(addr.listen(config).await?);
        listeners.insert(key, listener.clone());
        Ok(listener)
    }

    pub async fn release(&self, addr: &NetworkAddress) {
        let key = addr.to_string();
        let mut listeners = self.listeners.write().await;
        listeners.remove(&key);
    }
}
```

### 4. Certificate Cache

```rust
use rustls::{Certificate, PrivateKey};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use time::OffsetDateTime;

/// Cached certificate
pub struct CachedCertificate {
    pub cert: rustls::Certificate,
    pub key: rustls::PrivateKey,
    pub chain: Vec<rustls::Certificate>,
    pub names: Vec<String>,
    pub not_before: OffsetDateTime,
    pub not_after: OffsetDateTime,
    pub managed: bool,
    pub ocsp_response: Option<Vec<u8>>,
    pub hash: String,  // SHA256 of cert for identification
}

impl CachedCertificate {
    pub fn needs_renewal(&self, renewal_window_ratio: f64) -> bool {
        let lifetime = self.not_after - self.not_before;
        let renewal_window = lifetime * renewal_window_ratio;
        let time_remaining = self.not_after - OffsetDateTime::now_utc();
        time_remaining < renewal_window
    }

    pub fn expired(&self) -> bool {
        OffsetDateTime::now_utc() > self.not_after
    }
}

/// Certificate cache
pub struct CertificateCache {
    cache: Arc<RwLock<HashMap<String, Arc<CachedCertificate>>>>,
    name_index: Arc<RwLock<HashMap<String, Vec<String>>>>,  // name -> cert hashes
    options: CacheOptions,
    stop_signal: tokio::sync::Notify,
}

pub struct CacheOptions {
    pub capacity: usize,
    pub renewal_check_interval: Duration,
    pub ocsp_check_interval: Duration,
}

impl Default for CacheOptions {
    fn default() -> Self {
        Self {
            capacity: 10_000,
            renewal_check_interval: Duration::from_secs(3600),  // 1 hour
            ocsp_check_interval: Duration::from_secs(3600),      // 1 hour
        }
    }
}

impl CertificateCache {
    pub fn new(options: CacheOptions) -> Self {
        let cache = Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            name_index: Arc::new(RwLock::new(HashMap::new())),
            options,
            stop_signal: tokio::sync::Notify::new(),
        };

        // Start maintenance background task
        tokio::spawn(cache.maintenance_loop());

        cache
    }

    pub async fn insert(&self, cert: CachedCertificate) -> Result<(), CacheError> {
        let mut cache = self.cache.write().await;

        // Check capacity and evict if needed (LRU)
        if cache.len() >= self.options.capacity {
            self.evict_one(&mut cache).await?;
        }

        let hash = cert.hash.clone();
        let names = cert.names.clone();
        let cert_arc = Arc::new(cert);

        cache.insert(hash.clone(), cert_arc);

        // Update name index
        let mut name_index = self.name_index.write().await;
        for name in names {
            name_index
                .entry(name)
                .or_insert_with(Vec::new)
                .push(hash.clone());
        }

        Ok(())
    }

    pub async fn get_for_name(&self, name: &str) -> Option<Arc<CachedCertificate>> {
        let name_index = self.name_index.read().await;
        let cert_hashes = name_index.get(name)?;

        // Return first non-expired certificate
        let cache = self.cache.read().await;
        for hash in cert_hashes {
            if let Some(cert) = cache.get(hash) {
                if !cert.expired() {
                    return Some(cert.clone());
                }
            }
        }
        None
    }

    async fn maintenance_loop(&self) {
        let mut renewal_interval = tokio::time::interval(self.options.renewal_check_interval);
        let mut ocsp_interval = tokio::time::interval(self.options.ocsp_check_interval);

        loop {
            tokio::select! {
                _ = renewal_interval.tick() => {
                    self.check_renewals().await;
                }
                _ = ocsp_interval.tick() => {
                    self.update_ocsp_staples().await;
                }
                _ = self.stop_signal.notified() => {
                    break;
                }
            }
        }
    }

    async fn check_renewals(&self) {
        let cache = self.cache.read().await;
        for cert in cache.values() {
            if cert.managed && cert.needs_renewal(1.0 / 3.0) {
                // Queue for renewal
                // This would trigger the ACME renewal flow
                tracing::info!(
                    names = ?cert.names,
                    "Certificate needs renewal"
                );
            }
        }
    }

    async fn update_ocsp_staples(&self) {
        // Fetch and update OCSP responses
        // See OCSP section below
    }

    async fn evict_one(&self, cache: &mut HashMap<String, Arc<CachedCertificate>>) -> Result<(), CacheError> {
        // LRU eviction: remove oldest accessed certificate
        // Could use chrono::DateTime for last_accessed
        todo!()
    }
}
```

### 5. ACME Client Implementation

```rust
use acme2::{
    Authorization,
    Challenge,
    Directory,
    Order,
};
use rcgen::{
    Certificate,
    CertificateParams,
    DistinguishedName,
};
use ring::rand::SystemRandom;
use std::time::Duration;

/// ACME issuer configuration
pub struct AcmeIssuer {
    directory_url: String,
    email: String,
    caa_identities: Vec<String>,
    external_account: Option<ExternalAccount>,
    challenges: ChallengeConfig,
}

pub struct ChallengeConfig {
    http_challenge: bool,
    tls_alpn_challenge: bool,
    dns_challenge: Option<Arc<dyn DnsProvider>>,
    alternate_ports: AlternatePorts,
}

pub struct AlternatePorts {
    http: Option<u16>,
    tls_alpn: Option<u16>,
}

/// ACME client wrapper
pub struct AcmeClient {
    directory: Directory,
    account: acme2::Account,
    http: reqwest::Client,
}

impl AcmeClient {
    pub async fn new(issuer: &AcmeIssuer) -> Result<Self, AcmeError> {
        let directory = Directory::discover(
            &issuer.directory_url,
            Some(&issuer.http_client()),
        ).await?;

        // Create or retrieve account
        let account = directory
            .account(Email(issuer.email.clone()))
            .await?;

        Ok(Self {
            directory,
            account,
            http: reqwest::Client::new(),
        })
    }

    pub async fn obtain_certificate(
        &self,
        domains: &[String],
        private_key: &rustls::PrivateKey,
    ) -> Result<ObtainedCertificate, AcmeError> {
        // 1. Create order
        let order = self
            .account
            .new_order(&OrderRequest {
                identifiers: domains
                    .iter()
                    .map(|d| Identifier::Dns(d.clone()))
                    .collect(),
            })
            .await?;

        // 2. Authorize each domain
        let authorizations = order.authorizations().await?;
        for auth in &authorizations {
            self.complete_authorization(auth).await?;
        }

        // 3. Generate CSR
        let csr = self.generate_csr(domains, private_key)?;

        // 4. Finalize order
        let order = order.finalize(&csr).await?;

        // 5. Wait for certificate
        let cert = order.wait(Duration::from_secs(30)).await?;

        // 6. Download certificate
        let cert_response = cert.download().await?;

        Ok(ObtainedCertificate {
            certificate: cert_response.certs,
            issuer: cert_response.issuer,
            private_key: private_key.clone(),
        })
    }

    async fn complete_authorization(
        &self,
        auth: &Authorization,
    ) -> Result<(), AcmeError> {
        // Select challenge type
        let challenge = self.select_challenge(auth)?;

        match challenge {
            Challenge::Http01(http) => {
                // HTTP-01 challenge
                self.solve_http_challenge(http).await?;
            }
            Challenge::TlsAlpn01(tls) => {
                // TLS-ALPN-01 challenge
                self.solve_tls_alpn_challenge(tls).await?;
            }
            Challenge::Dns01(dns) => {
                // DNS-01 challenge
                self.solve_dns_challenge(dns).await?;
            }
        }

        Ok(())
    }

    fn generate_csr(
        &self,
        domains: &[String],
        private_key: &rustls::PrivateKey,
    ) -> Result<Vec<u8>, AcmeError> {
        let mut params = CertificateParams::default();

        // Add SANs
        for domain in domains {
            params
                .subject_alt_names
                .push(rcgen::SanType::DnsName(domain.clone()));
        }

        // Generate certificate
        let cert = Certificate::from_params(params)?;
        let csr = cert.serialize_request()?;

        Ok(csr.der().as_ref().to_vec())
    }
}

/// HTTP-01 Challenge Solver
pub struct HttpChallengeSolver {
    listener: Arc<tokio::sync::Mutex<Option<HttpChallengeServer>>>,
}

struct HttpChallengeServer {
    listener: tokio::net::TcpListener,
    challenges: HashMap<String, String>,  // token -> key_authorization
}

impl HttpChallengeSolver {
    pub async fn present(
        &self,
        token: &str,
        key_authorization: &str,
        port: u16,
    ) -> Result<(), ChallengeError> {
        let mut server = self.listener.lock().await;

        if server.is_none() {
            // Start HTTP server
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
                .await?;

            let mut new_server = HttpChallengeServer {
                listener,
                challenges: HashMap::new(),
            };
            new_server.challenges.insert(token.to_string(), key_authorization.to_string());

            let server_clone = Arc::new(tokio::sync::Mutex::new(new_server));
            tokio::spawn(self.serve_challenges(server_clone.clone()));

            *server = Some(HttpChallengeServer {
                listener: server_clone.lock().await.listener.clone(),
                challenges: HashMap::new(),
            });
        } else {
            // Add challenge to existing server
            server.as_mut().unwrap().challenges.insert(
                token.to_string(),
                key_authorization.to_string()
            );
        }

        Ok(())
    }

    async fn serve_challenges(&self, server: Arc<tokio::sync::Mutex<HttpChallengeServer>>) {
        loop {
            let (stream, addr) = match server.lock().await.listener.accept().await {
                Ok(result) => result,
                Err(_) => break,  // Server closed
            };

            tokio::spawn(async move {
                if let Err(e) = self.handle_challenge_request(stream, &server).await {
                    tracing::error!("HTTP challenge error: {}", e);
                }
            });
        }
    }

    async fn handle_challenge_request(
        &self,
        mut stream: tokio::net::TcpStream,
        server: &Arc<tokio::sync::Mutex<HttpChallengeServer>>,
    ) -> Result<(), std::io::Error> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).await?;

        // Parse request
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(());
        }

        let method = parts[0];
        let path = parts[1];

        // Check if it's a challenge request
        if method != "GET" || !path.starts_with("/.well-known/acme-challenge/") {
            // Return 404 for non-challenge requests
            let response = "HTTP/1.1 404 Not Found\r\n\r\n";
            reader.get_mut().write_all(response.as_bytes()).await?;
            return Ok(());
        }

        // Extract token
        let token = path.trim_start_matches("/.well-known/acme-challenge/");

        // Look up key authorization
        let key_auth = {
            let server = server.lock().await;
            server.challenges.get(token).cloned()
        };

        let response = if let Some(ka) = key_auth {
            format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/plain\r\n\
                 Content-Length: {}\r\n\
                 \r\n\
                 {}",
                ka.len(),
                ka
            )
        } else {
            "HTTP/1.1 404 Not Found\r\n\r\n".to_string()
        };

        reader.get_mut().write_all(response.as_bytes()).await?;
        Ok(())
    }
}
```

### 6. TLS Integration

```rust
use rustls::{
    ServerConfig,
    Certificate,
    PrivateKey,
    server::ResolvesServerCert,
    sign::CertifiedKey,
};
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

/// Certificate resolver for TLS handshakes
pub struct TlsCertificateResolver {
    cache: Arc<CertificateCache>,
    default_cert: Option<Arc<CertifiedKey>>,
}

impl TlsCertificateResolver {
    pub fn new(cache: Arc<CertificateCache>) -> Self {
        Self {
            cache,
            default_cert: None,
        }
    }

    pub fn set_default_cert(&mut self, cert: Arc<CertifiedKey>) {
        self.default_cert = Some(cert);
    }
}

impl ResolvesServerCert for TlsCertificateResolver {
    fn resolve(&self, client_hello: rustls::server::ClientHello) -> Option<Arc<CertifiedKey>> {
        let server_name = client_hello.server_name()?;

        // Try exact match first
        if let Some(cert) = self.cache.get_for_name_sync(server_name) {
            return Some(cert.certified_key.clone());
        }

        // Try wildcard match
        let wildcard_name = format!("*.{}", server_name.splitn(2, '.').nth(1)?);
        if let Some(cert) = self.cache.get_for_name_sync(&wildcard_name) {
            return Some(cert.certified_key.clone());
        }

        // Fall back to default
        self.default_cert.clone()
    }
}

/// Create TLS configuration
pub fn create_tls_config(
    cache: Arc<CertificateCache>,
) -> Result<Arc<ServerConfig>, TlsError> {
    let resolver = TlsCertificateResolver::new(cache.clone());

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));

    // Enable OCSP stapling
    config.verifier = Arc::new(
        rustls::client::WebPkiServerVerifier::builder(
            rustls::RootCertStore::empty()
        )
        .build()
        .unwrap()
    );

    Ok(Arc::new(config))
}

/// TLS Acceptor wrapper
pub struct TlsListener {
    inner: tokio::net::TcpListener,
    acceptor: TlsAcceptor,
}

impl TlsListener {
    pub async fn accept(&self) -> Result<tokio_rustls::server::TlsStream<tokio::net::TcpStream>, std::io::Error> {
        let (stream, addr) = self.inner.accept().await?;
        self.acceptor.accept(stream).await
    }
}
```

### 7. OCSP Stapling

```rust
use x509_parser::ocsp::{
    OcspResponse,
    OcspResponseStatus,
    OcspSingleResponse,
};
use reqwest::Client;

/// OCSP stapler
pub struct OcspStapler {
    http_client: Client,
}

impl OcspStapler {
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }

    pub async fn fetch_ocsp_response(
        &self,
        cert: &rustls::Certificate,
        issuer_cert: &rustls::Certificate,
    ) -> Result<Vec<u8>, OcspError> {
        // Parse certificate to get OCSP responder URL
        let ocsp_urls = cert.0.ocsp_responders;
        if ocsp_urls.is_empty() {
            return Err(OcspError::NoOcspResponder);
        }

        // Build OCSP request
        let ocsp_request = self.build_ocsp_request(cert, issuer_cert)?;

        // Send to responder
        let response = self
            .http_client
            .post(&ocsp_urls[0])
            .header("Content-Type", "application/ocsp-request")
            .body(ocsp_request.to_der()?)
            .send()
            .await?
            .bytes()
            .await?;

        // Parse response
        let ocsp_response = OcspResponse::from_der(&response)?;

        // Verify response
        self.verify_ocsp_response(&ocsp_response, issuer_cert)?;

        Ok(response.to_vec())
    }

    fn build_ocsp_request(
        &self,
        cert: &rustls::Certificate,
        issuer_cert: &rustls::Certificate,
    ) -> Result<ocsp::OcspRequest, OcspError> {
        use ocsp::{OcspCertId, OcspRequestBuilder};

        // Build certificate ID
        let cert_id = OcspCertId::from_cert(cert, issuer_cert)?;

        // Build request
        let request = OcspRequestBuilder::new()
            .add_cert(cert_id)
            .build()?;

        Ok(request)
    }

    fn verify_ocsp_response(
        &self,
        response: &OcspResponse,
        issuer_cert: &rustls::Certificate,
    ) -> Result<(), OcspError> {
        // Check response status
        match response.response_status {
            OcspResponseStatus::Successful => {}
            _ => return Err(OcspError::BadResponseStatus),
        }

        // Verify signature
        response.verify_signature(&issuer_cert.0)?;

        // Check thisUpdate/nextUpdate
        let now = chrono::Utc::now();
        for single_response in &response.responses {
            if single_response.this_update > now {
                return Err(OcspError::ResponseNotYetValid);
            }
            if let Some(next_update) = single_response.next_update {
                if next_update < now {
                    return Err(OcspError::ResponseExpired);
                }
            }
        }

        Ok(())
    }

    pub fn is_response_fresh(
        &self,
        response: &OcspResponse,
        buffer_ratio: f64,
    ) -> bool {
        for single_response in &response.responses {
            let this_update = single_response.this_update;
            let next_update = match single_response.next_update {
                Some(nu) => nu,
                None => return false,
            };

            let validity_period = next_update - this_update;
            let buffer = validity_period * buffer_ratio;

            if chrono::Utc::now() + buffer > next_update {
                return false;  // Not fresh
            }
        }

        true
    }
}
```

### 8. Storage Abstraction

```rust
use async_trait::async_trait;
use std::time::SystemTime;

/// Storage backend trait
#[async_trait]
pub trait Storage: Send + Sync {
    /// Store a value
    async fn store(&self, key: &str, value: &[u8]) -> Result<(), StorageError>;

    /// Load a value
    async fn load(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// Delete a value
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// Check if key exists
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;

    /// List keys with prefix
    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError>;

    /// Get metadata about a key
    async fn stat(&self, key: &str) -> Result<KeyInfo, StorageError>;

    /// Acquire a distributed lock
    async fn lock(&self, key: &str) -> Result<Box<dyn Lock>, StorageError>;
}

#[derive(Debug)]
pub struct KeyInfo {
    pub key: String,
    pub size: u64,
    pub modified: SystemTime,
}

/// Distributed lock trait
#[async_trait]
pub trait Lock: Send + Sync {
    /// Release the lock
    async fn unlock(&self) -> Result<(), StorageError>;
}

/// File system storage implementation
pub struct FileStorage {
    root_path: std::path::PathBuf,
}

impl FileStorage {
    pub fn new(root_path: &str) -> Self {
        Self {
            root_path: std::path::PathBuf::from(root_path),
        }
    }

    fn key_to_path(&self, key: &str) -> std::path::PathBuf {
        self.root_path.join(key)
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn store(&self, key: &str, value: &[u8]) -> Result<(), StorageError> {
        let path = self.key_to_path(key);

        // Create parent directories
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write atomically (write to temp file, then rename)
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, value).await?;
        tokio::fs::rename(&temp_path, &path).await?;

        Ok(())
    }

    async fn load(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let path = self.key_to_path(key);
        tokio::fs::read(&path).await.map_err(StorageError::from)
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = self.key_to_path(key);
        tokio::fs::remove_file(&path).await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let path = self.key_to_path(key);
        Ok(tokio::fs::metadata(&path).await.is_ok())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        use tokio_stream::StreamExt;

        let prefix_path = self.key_to_path(prefix);
        let mut keys = Vec::new();

        let mut entries = tokio::fs::read_dir(&prefix_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(relative) = path.strip_prefix(&self.root_path).ok() {
                keys.push(relative.to_string_lossy().to_string());
            }
        }

        Ok(keys)
    }

    async fn stat(&self, key: &str) -> Result<KeyInfo, StorageError> {
        let path = self.key_to_path(key);
        let metadata = tokio::fs::metadata(&path).await?;

        Ok(KeyInfo {
            key: key.to_string(),
            size: metadata.len(),
            modified: metadata.modified()?,
        })
    }

    async fn lock(&self, key: &str) -> Result<Box<dyn Lock>, StorageError> {
        // Use flock for file-based locking
        let lock_path = self.root_path.join("locks").join(format!("{}.lock", key));

        // Create lock directory
        tokio::fs::create_dir_all(lock_path.parent().unwrap()).await?;

        // Open/create lock file
        let file = tokio::fs::File::create(&lock_path).await?;

        // Acquire exclusive lock
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = file.as_raw_fd();
            unsafe {
                if libc::flock(fd, libc::LOCK_EX) != 0 {
                    return Err(StorageError::LockFailed);
                }
            }
        }

        Ok(Box::new(FileLock { file, path: lock_path }))
    }
}

struct FileLock {
    file: tokio::fs::File,
    path: std::path::PathBuf,
}

#[async_trait]
impl Lock for FileLock {
    async fn unlock(&self) -> Result<(), StorageError> {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = self.file.as_raw_fd();
            unsafe {
                libc::flock(fd, libc::LOCK_UN);
            }
        }

        // Clean up lock file
        tokio::fs::remove_file(&self.path).await?;

        Ok(())
    }
}
```

### 9. HTTP Challenge Handler Integration

```rust
use hyper::{Body, Request, Response, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global challenge state
pub struct ChallengeState {
    challenges: RwLock<HashMap<String, ChallengeInfo>>,
}

pub struct ChallengeInfo {
    token: String,
    key_authorization: String,
    created_at: Instant,
}

impl ChallengeState {
    pub fn new() -> Self {
        Self {
            challenges: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_challenge(&self, token: String, key_authorization: String) {
        let mut challenges = self.challenges.write().await;
        challenges.insert(token.clone(), ChallengeInfo {
            token,
            key_authorization,
            created_at: Instant::now(),
        });
    }

    pub async fn remove_challenge(&self, token: &str) {
        let mut challenges = self.challenges.write().await;
        challenges.remove(token);
    }

    pub async fn get_challenge(&self, token: &str) -> Option<String> {
        let challenges = self.challenges.read().await;
        challenges.get(token).map(|c| c.key_authorization.clone())
    }
}

/// HTTP handler that serves ACME challenges
pub struct AcmeChallengeHandler {
    state: Arc<ChallengeState>,
}

impl AcmeChallengeHandler {
    pub fn new(state: Arc<ChallengeState>) -> Self {
        Self { state }
    }
}

impl hyper::service::Service<Request<Body>> for AcmeChallengeHandler {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Body>) -> Self::Future {
        let state = self.state.clone();

        Box::pin(async move {
            // Check if this is an ACME challenge request
            if req.uri().path().starts_with("/.well-known/acme-challenge/") {
                let token = req.uri().path().trim_start_matches("/.well-known/acme-challenge/");

                if let Some(key_auth) = state.get_challenge(token).await {
                    return Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "text/plain")
                        .body(Body::from(key_auth))
                        .unwrap());
                }
            }

            // Not a challenge request - return 404
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap())
        })
    }
}
```

## Production Considerations

### Error Handling Strategy

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcmeError {
    #[error("ACME protocol error: {0}")]
    Protocol(#[from] acme2::Error),

    #[error("Certificate error: {0}")]
    Certificate(#[from] rcgen::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Challenge failed: {0}")]
    ChallengeFailed(String),

    #[error("Rate limited by CA")]
    RateLimited,

    #[error("Account does not exist")]
    AccountNotFound,
}

// Use anyhow for application-level errors
pub type Result<T> = std::result::Result<T, anyhow::Error>;
```

### Observability

```rust
use tracing::{info, warn, error, debug, instrument};
use tracing_subscriber::{layer::SubscriberExt, Registry};

pub fn init_logging() {
    let subscriber = Registry::default()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env());

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
}

#[instrument(skip_all, fields(domain = %domain))]
async fn obtain_certificate(domain: &str) -> Result<Certificate> {
    info!("Starting certificate obtainment");

    // ... implementation ...

    info!("Certificate obtained successfully");
    Ok(cert)
}
```

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;

    #[tokio::test]
    async fn test_certificate_cache() {
        let cache = CertificateCache::new(CacheOptions::default());

        // Insert test certificate
        let cert = create_test_cert();
        cache.insert(cert).await.unwrap();

        // Verify retrieval
        let retrieved = cache.get_for_name("example.com").await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_http_challenge_handler() {
        let state = Arc::new(ChallengeState::new());
        state.add_challenge("token123".to_string(), "keyauth456".to_string()).await;

        let handler = AcmeChallengeHandler::new(state);

        // Test challenge request
        let req = Request::builder()
            .uri("/.well-known/acme-challenge/token123")
            .body(Body::empty())
            .unwrap();

        let response = handler.call(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
```

## Implementation Roadmap

### Phase 1: Core Infrastructure (2-3 months)
1. Module system and configuration parsing
2. Listener management
3. Basic HTTP server with hyper
4. TLS integration with rustls

### Phase 2: ACME Integration (2-3 months)
1. ACME client implementation
2. HTTP-01 challenge solver
3. TLS-ALPN-01 challenge solver
4. Certificate storage and caching

### Phase 3: Advanced Features (2-3 months)
1. OCSP stapling
2. Automatic certificate renewal
3. DNS-01 challenge with libdns equivalent
4. Distributed solving support

### Phase 4: Production Hardening (2-3 months)
1. Comprehensive error handling
2. Observability (metrics, tracing)
3. Performance optimization
4. Security audit

### Phase 5: Ecosystem (ongoing)
1. Plugin system for extensions
2. Storage backend implementations
3. Management CLI
4. Documentation

## Key Differences from Go

| Aspect | Go (Caddy) | Rust |
|--------|------------|------|
| Concurrency | Goroutines | Async/await (tokio) |
| Memory management | GC | Ownership/borrowing |
| Dynamic dispatch | Interfaces | Trait objects |
| Reflection | Yes (reflect) | Limited (Any trait) |
| Error handling | Errors as values | Result/Option |
| Compilation | Fast | Slower but improving |
| Binary size | Larger | Typically smaller |
| Startup time | Fast | Very fast (no GC) |

## Risk Mitigation

1. **ACME Protocol Changes**: Use established crates like acme2 that maintain RFC compliance
2. **TLS Security**: Use rustls (audited) instead of OpenSSL bindings
3. **Memory Safety**: Leverage Rust's type system, avoid unsafe blocks
4. **Performance**: Profile early, use async I/O throughout
5. **Compatibility**: Test against multiple ACME CAs (Let's Encrypt, ZeroSSL, etc.)

## Conclusion

Reproducing Caddy in Rust is achievable with careful architecture and the right crate selection. The Rust ecosystem provides most building blocks needed. Key challenges are the ACME implementation, TLS integration, and distributed coordination - but all are solvable with existing primitives.

The main advantages of a Rust implementation would be:
- Lower memory footprint (no GC)
- Faster startup time
- Stronger guarantees around thread safety
- Potentially better performance for CPU-intensive operations

The main challenges:
- More complex module system (no runtime reflection)
- Steeper learning curve for contributors
- Smaller ecosystem for some ACME/TLS features
