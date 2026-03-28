# Caddy Rust Revision - Complete Translation Guide

**Location:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/caddy/caddy/`
**Source:** Caddy web server (Go)
**Target:** Rust with valtron executor (no tokio for Lambda deployment)
**Date:** 2026-03-27

---

## Table of Contents

1. [Overview](#1-overview)
2. [Type System Design](#2-type-system-design)
3. [Module System Translation](#3-module-system-translation)
4. [TLS/ACME Translation](#4-tlsacme-translation)
5. [HTTP Server Translation](#5-http-server-translation)
6. [Reverse Proxy Translation](#6-reverse-proxy-translation)
7. [Configuration Parsing](#7-configuration-parsing)
8. [Valtron Integration](#8-valtron-integration)

---

## 1. Overview

### 1.1 What We're Translating

Caddy is a web server platform with:
- Automatic HTTPS via ACME
- Module-based architecture
- Reverse proxy with load balancing
- Caddyfile configuration language

### 1.2 Translation Goals

| Goal | Approach |
|------|----------|
| No async/await | Use valtron TaskIterator pattern |
| No tokio (Lambda) | Standard library + hyper (without rt) |
| Module system | Trait objects + registry |
| TLS automation | `rcgen` + ACME HTTP client |
| Configuration | Custom parser or serde |

### 1.3 Key Design Decisions

#### Ownership Strategy

```rust
// Go uses garbage collection - references are free
type Handler struct {
    Transport http.RoundTripper  // Interface reference
    Upstreams []*Upstream        // Slice of pointers
}

// Rust uses explicit ownership
pub struct Handler {
    transport: Arc<dyn Transport>,     // Shared ownership
    upstreams: Arc<RwLock<UpstreamPool>>, // Shared mutable state
}
```

#### Executor Choice

```rust
// For Lambda deployment (no tokio runtime)
// Use valtron's single-threaded executor

use foundation_core::valtron::single::{spawn, run_until_complete};

// For standard deployment (tokio allowed)
// Use hyper with full async

use tokio::net::TcpListener;
use hyper::server::conn::http1;
```

---

## 2. Type System Design

### 2.1 Core Module Types

```rust
use std::any::Any;
use std::sync::Arc;

/// Module ID like Caddy's: "http.handlers.reverse_proxy"
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ModuleId(String);

impl ModuleId {
    pub fn namespace(&self) -> &str {
        self.0.rsplit_once('.').map(|(ns, _)| ns).unwrap_or("")
    }

    pub fn name(&self) -> &str {
        self.0.rsplit_once('.').map(|(_, name)| name).unwrap_or(&self.0)
    }
}

/// Base module trait - all modules implement this
pub trait Module: Send + Sync {
    fn module_id(&self) -> ModuleId;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Optional: Provisioning (setup after config loaded)
pub trait Provisioner: Module {
    fn provision(&mut self, ctx: &Context) -> Result<(), ModuleError>;
}

/// Optional: Validation (validate config before use)
pub trait Validator: Module {
    fn validate(&self) -> Result<(), ModuleError>;
}

/// Optional: Cleanup (called on shutdown/reload)
pub trait CleanerUpper: Module {
    fn cleanup(&mut self) -> Result<(), ModuleError>;
}
```

### 2.2 Module Registry

```rust
use std::collections::HashMap;
use std::sync::{RwLock, OnceLock};

/// Module constructor function
type ModuleConstructor = Box<dyn Fn() -> Box<dyn Module> + Send + Sync>;

/// Global module registry
pub struct ModuleRegistry {
    modules: RwLock<HashMap<ModuleId, ModuleConstructor>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
        }
    }

    pub fn register<M: Module + Default + 'static>(&self) {
        let id = M::default().module_id();
        let constructor: ModuleConstructor = Box::new(|| Box::new(M::default()));

        let mut modules = self.modules.write().unwrap();
        if modules.contains_key(&id) {
            panic!("Module already registered: {}", id.0);
        }
        modules.insert(id, constructor);
    }

    pub fn get(&self, id: &ModuleId) -> Option<Box<dyn Module>> {
        let modules = self.modules.read().unwrap();
        modules.get(id).map(|ctor| ctor())
    }

    pub fn get_modules_in_namespace(&self, namespace: &str) -> Vec<ModuleId> {
        let modules = self.modules.read().unwrap();
        modules
            .keys()
            .filter(|id| id.namespace() == namespace)
            .cloned()
            .collect()
    }
}

// Global registry instance
static REGISTRY: OnceLock<ModuleRegistry> = OnceLock::new();

pub fn get_registry() -> &'static ModuleRegistry {
    REGISTRY.get_or_init(ModuleRegistry::new)
}

// Macro for easy registration
#[macro_export]
macro_rules! register_module {
    ($type:ty, $id:expr) => {
        impl $crate::module::Module for $type {
            fn module_id(&self) -> $crate::module::ModuleId {
                $crate::module::ModuleId($id.to_string())
            }
            fn as_any(&self) -> &dyn std::any::Any { self }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
        }

        $crate::inventory::submit! {
            $crate::module::ModuleRegistration::new($id, || Box::new(<$type>::default()))
        }
    };
}
```

### 2.3 HTTP Handler Types

```rust
use http::{Request, Response, StatusCode};
use bytes::Bytes;

/// HTTP Body type
pub type Body = Bytes;

/// HTTP Handler error
#[derive(Debug)]
pub enum HandlerError {
    UpstreamError(String),
    NotFound,
    InternalError(String),
    Timeout,
}

impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HandlerError::UpstreamError(e) => write!(f, "Upstream error: {}", e),
            HandlerError::NotFound => write!(f, "Not found"),
            HandlerError::InternalError(e) => write!(f, "Internal error: {}", e),
            HandlerError::Timeout => write!(f, "Timeout"),
        }
    }
}

/// Next handler in chain
pub struct Next {
    handler: Option<Arc<dyn HttpHandler>>,
}

impl Next {
    pub fn serve_http(
        &self,
        req: Request<Body>,
    ) -> impl Future<Output = Result<Response<Body>, HandlerError>> + Send {
        // Execute next handler in chain
        todo!()
    }
}

/// HTTP Middleware Handler trait
pub trait HttpHandler: Module {
    fn serve_http(
        &self,
        req: Request<Body>,
        next: Next,
    ) -> impl Future<Output = Result<Response<Body>, HandlerError>> + Send;
}

/// HTTP Matcher trait
pub trait HttpMatcher: Module {
    fn matches(&self, req: &Request<Body>) -> bool;
}
```

### 2.4 Context and Configuration

```rust
use std::collections::HashMap;
use serde_json::Value;

/// Configuration context (like Caddy's Context)
pub struct Context {
    apps: HashMap<String, Arc<dyn Module>>,
    storage: Arc<dyn Storage>,
    logger: Logger,
    config: Arc<Value>,
}

impl Context {
    /// Load a module from configuration
    pub fn load_module<M: Module + 'static>(
        &self,
        config: &Value,
    ) -> Result<Arc<M>, ModuleError> {
        // Extract module type
        let module_type = config["module"]
            .as_str()
            .ok_or(ModuleError::MissingModuleType)?;

        let module_id = ModuleId(module_type.to_string());

        // Get from registry
        let registry = get_registry();
        let mut module = registry
            .get(&module_id)
            .ok_or_else(|| ModuleError::NotFound(module_id.0.clone()))?;

        // Deserialize config into module
        let module_any = module.as_any_mut();
        let concrete = module_any
            .downcast_mut::<M>()
            .ok_or(ModuleError::TypeMismatch)?;

        // Provision
        if let Some(provisioner) = concrete.as_any_mut().downcast_mut::<dyn Provisioner>() {
            provisioner.provision(self)?;
        }

        // Validate
        if let Some(validator) = concrete.as_any().downcast_ref::<dyn Validator>() {
            validator.validate()?;
        }

        Ok(Arc::new(M::from(concrete)))
    }

    /// Get a loaded app
    pub fn app<A: Module + 'static>(&self, name: &str) -> Option<Arc<A>> {
        self.apps.get(name).and_then(|m| {
            m.as_any().downcast_ref::<A>().map(Arc::clone)
        })
    }
}
```

---

## 3. Module System Translation

### 3.1 Reverse Proxy Module

```rust
// Go:
// type Handler struct {
//     TransportRaw json.RawMessage
//     Upstreams UpstreamPool
//     HealthChecks *HealthChecks
// }

#[derive(Debug, Default)]
pub struct ReverseProxyHandler {
    /// Transport configuration (JSON)
    transport_config: serde_json::Value,

    /// Upstream pool
    upstreams: UpstreamPool,

    /// Health check configuration
    health_checks: Option<HealthChecksConfig>,

    /// Load balancing configuration
    load_balancing: Option<LoadBalancingConfig>,

    /// Runtime state
    transport: Option<Arc<dyn Transport>>,
    logger: Option<Logger>,
}

register_module!(ReverseProxyHandler, "http.handlers.reverse_proxy");

impl Provisioner for ReverseProxyHandler {
    fn provision(&mut self, ctx: &Context) -> Result<(), ModuleError> {
        self.logger = Some(ctx.logger.clone());

        // Load transport module
        if !self.transport_config.is_null() {
            let transport_type = self.transport_config["protocol"]
                .as_str()
                .unwrap_or("http");

            match transport_type {
                "http" => {
                    let config: HttpTransportConfig = serde_json::from_value(
                        self.transport_config.clone()
                    )?;
                    self.transport = Some(Arc::new(HttpTransport::new(config)));
                }
                "fastcgi" => {
                    // Load FastCGI transport
                    todo!()
                }
                _ => return Err(ModuleError::UnknownTransport(transport_type.to_string())),
            }
        }

        // Default to HTTP transport
        if self.transport.is_none() {
            self.transport = Some(Arc::new(HttpTransport::default()));
        }

        // Provision health checks
        if let Some(ref mut hc) = self.health_checks {
            hc.provision(ctx)?;
        }

        Ok(())
    }
}

impl HttpHandler for ReverseProxyHandler {
    async fn serve_http(
        &self,
        req: Request<Body>,
        _next: Next,
    ) -> Result<Response<Body>, HandlerError> {
        let start = std::time::Instant::now();
        let mut last_error: Option<HandlerError> = None;

        // Retry loop
        while start.elapsed() < self.load_balancing.try_duration {
            // Select upstream
            let upstream = self.select_backend(&req);
            if upstream.is_none() {
                last_error = Some(HandlerError::UpstreamError("No healthy upstreams".into()));
                break;
            }

            // Try to proxy
            match self.proxy_to_upstream(req, &upstream.unwrap()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    // Record for passive health check
                    upstream.as_ref().unwrap().record_failure(&e).await;
                    last_error = Some(e);

                    // Check if retryable
                    if !last_error.as_ref().unwrap().is_retryable() {
                        break;
                    }

                    // Wait before retry
                    tokio::time::sleep(self.load_balancing.retry_interval).await;
                }
            }
        }

        Err(last_error.unwrap_or(HandlerError::UpstreamError("No healthy upstreams".into())))
    }
}
```

### 3.2 Load Balancing

```rust
use std::sync::atomic::{AtomicU64, Ordering};

/// Load balancing selection policy
pub trait LoadBalancer: Send + Sync {
    fn select(&self, pool: &UpstreamPool) -> Option<Arc<Upstream>>;
}

/// Round Robin selection
#[derive(Debug, Default)]
pub struct RoundRobin {
    counter: AtomicU64,
}

impl LoadBalancer for RoundRobin {
    fn select(&self, pool: &UpstreamPool) -> Option<Arc<Upstream>> {
        let available = pool.healthy_upstreams();
        if available.is_empty() {
            return None;
        }

        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        Some(available[(n as usize) % available.len()].clone())
    }
}

/// Least Connections selection
#[derive(Debug, Default)]
pub struct LeastConnections;

impl LoadBalancer for LeastConnections {
    fn select(&self, pool: &UpstreamPool) -> Option<Arc<Upstream>> {
        pool.healthy_upstreams()
            .into_iter()
            .min_by_key(|u| u.active_requests.load(Ordering::Relaxed))
    }
}

/// IP Hash selection
#[derive(Debug, Default)]
pub struct IpHash;

impl LoadBalancer for IpHash {
    fn select(&self, pool: &UpstreamPool) -> Option<Arc<Upstream>> {
        // Implementation uses request IP
        todo!()
    }
}

/// Load balancing configuration
#[derive(Debug, Clone)]
pub struct LoadBalancingConfig {
    pub try_duration: Duration,
    pub retry_interval: Duration,
    pub max_connections: Option<usize>,
    pub selection_policy: String,  // "round_robin", "least_conn", etc.
}
```

### 3.3 Health Checks

```rust
/// Upstream with health tracking
pub struct Upstream {
    pub dial: String,
    pub host: String,
    pub port: u16,

    // Health state
    healthy: AtomicBool,
    failures: AtomicUsize,
    active_requests: AtomicUsize,
    last_fail: RwLock<Option<Instant>>,
}

impl Upstream {
    pub fn new(dial: String) -> Self {
        let (host, port) = Self::parse_dial(&dial);
        Self {
            dial,
            host,
            port,
            healthy: AtomicBool::new(true),
            failures: AtomicUsize::new(0),
            active_requests: AtomicUsize::new(0),
            last_fail: RwLock::new(None),
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    pub fn mark_healthy(&self) {
        self.healthy.store(true, Ordering::Relaxed);
        self.failures.store(0, Ordering::Relaxed);
    }

    pub fn mark_unhealthy(&self) {
        self.healthy.store(false, Ordering::Relaxed);
        *self.last_fail.write().unwrap() = Some(Instant::now());
    }

    pub async fn record_failure(&self, error: &HandlerError) {
        let failures = self.failures.fetch_add(1, Ordering::Relaxed) + 1;

        if failures >= 3 {
            self.mark_unhealthy();
        }
    }

    pub fn start_request(&self) {
        self.active_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn end_request(&self) {
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Active health checker
pub struct ActiveHealthChecker {
    path: String,
    interval: Duration,
    timeout: Duration,
    expect_status: StatusCode,
}

impl ActiveHealthChecker {
    pub fn spawn_checker(
        &self,
        upstreams: Arc<RwLock<UpstreamPool>>,
    ) -> JoinHandle<()> {
        let path = self.path.clone();
        let interval = self.interval;
        let timeout = self.timeout;
        let expect = self.expect_status;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                let upstreams_guard = upstreams.read().unwrap();
                for upstream in upstreams_guard.iter() {
                    let upstream = upstream.clone();
                    let path = path.clone();

                    tokio::spawn(async move {
                        let healthy = Self::check_single(&upstream, &path, timeout, expect).await;
                        if healthy {
                            upstream.mark_healthy();
                        } else {
                            upstream.mark_unhealthy();
                        }
                    });
                }
            }
        })
    }

    async fn check_single(
        upstream: &Upstream,
        path: &str,
        timeout: Duration,
        expect: StatusCode,
    ) -> bool {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap();

        let url = format!("http://{}:{}{}", upstream.host, upstream.port, path);

        match client.get(&url).send().await {
            Ok(resp) => resp.status() == expect,
            Err(_) => false,
        }
    }
}
```

---

## 4. TLS/ACME Translation

### 4.1 Certificate Issuer Trait

```rust
use x509_parser::prelude::*;
use rcgen::{CertificateParams, KeyPair};

/// Certificate for TLS
#[derive(Debug, Clone)]
pub struct Certificate {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
    pub issuer: String,
    pub not_before: chrono::DateTime<chrono::Utc>,
    pub not_after: chrono::DateTime<chrono::Utc>,
    pub subjects: Vec<String>,
}

/// Certificate issuance request
#[derive(Debug)]
pub struct IssuanceRequest {
    pub sans: Vec<String>,  // Subject Alternative Names
    pub not_after: Option<chrono::DateTime<chrono::Utc>>,
}

/// Certificate issuer trait (like certmagic.Issuer)
pub trait CertificateIssuer: Send + Sync {
    fn issue(
        &self,
        ctx: &TlsContext,
        request: IssuanceRequest,
    ) -> impl Future<Output = Result<Certificate, TlsError>> + Send;

    fn revoke(
        &self,
        ctx: &TlsContext,
        cert: &Certificate,
    ) -> impl Future<Output = Result<(), TlsError>> + Send;
}
```

### 4.2 ACME Issuer

```rust
#[derive(Debug)]
pub struct AcmeIssuer {
    ca_url: String,
    email: String,
    account: RwLock<Option<AcmeAccount>>,
    http_client: reqwest::Client,
    storage: Arc<dyn CertificateStorage>,
}

impl AcmeIssuer {
    pub fn new(ca_url: String, email: String, storage: Arc<dyn CertificateStorage>) -> Self {
        Self {
            ca_url,
            email,
            account: RwLock::new(None),
            http_client: reqwest::Client::new(),
            storage,
        }
    }

    async fn get_or_create_account(&self) -> Result<AcmeAccount, TlsError> {
        // Check cache
        {
            let guard = self.account.read().unwrap();
            if let Some(acc) = guard.as_ref() {
                return Ok(acc.clone());
            }
        }

        // Try load from storage
        if let Some(acc_data) = self.storage.load("account.json").await? {
            let acc: AcmeAccount = serde_json::from_slice(&acc_data)?;
            *self.account.write().unwrap() = Some(acc.clone());
            return Ok(acc);
        }

        // Create new account
        let key = P256PrivateKey::generate();
        let account = self.register_account(&key).await?;

        // Cache
        *self.account.write().unwrap() = Some(account.clone());
        self.storage.store("account.json", &serde_json::to_vec(&account)?).await?;

        Ok(account)
    }

    async fn register_account(&self, key: &P256PrivateKey) -> Result<AcmeAccount, TlsError> {
        let directory = self.fetch_directory().await?;

        let payload = serde_json::json!({
            "contact": [format!("mailto:{}", self.email)],
            "termsOfServiceAgreed": true,
        });

        let response = self.post_acme(&directory.new_account, key, &payload).await?;
        let account: AcmeAccount = response.json().await?;

        Ok(account)
    }
}

impl CertificateIssuer for AcmeIssuer {
    async fn issue(
        &self,
        ctx: &TlsContext,
        request: IssuanceRequest,
    ) -> Result<Certificate, TlsError> {
        let account = self.get_or_create_account().await?;
        let directory = self.fetch_directory().await?;

        // Create order
        let order_payload = serde_json::json!({
            "identifiers": request.sans.iter().map(|d| {
                serde_json::json!({ "type": "dns", "value": d })
            }).collect::<Vec<_>>(),
        });

        let order: AcmeOrder = self.post_acme(
            &directory.new_order,
            &account.key,
            &order_payload
        ).await?.json().await?;

        // Solve challenges for each authorization
        for auth_url in &order.authorizations {
            let auth: AcmeAuthorization = self.post_acme(auth_url, &account.key, &()).await?.json().await?;

            // Find HTTP-01 challenge
            let challenge = auth.challenges.iter()
                .find(|c| c.type == "http-01")
                .ok_or(TlsError::NoSupportedChallenge)?;

            // Get key authorization
            let key_auth = self.compute_key_authorization(&account.key, &challenge.token);

            // Register challenge response
            ctx.register_http_challenge(&challenge.token, &key_auth).await?;

            // Tell ACME server to verify
            self.post_acme(&challenge.url, &account.key, &()).await?;

            // Wait for validation
            self.wait_for_challenge(&challenge.url).await?;

            // Clean up
            ctx.unregister_http_challenge(&challenge.token).await?;
        }

        // Finalize order
        let csr = self.generate_csr(&request.sans).await?;
        self.post_acme(
            &order.finalize,
            &account.key,
            &serde_json::json!({ "csr": base64_url::encode(&csr) })
        ).await?;

        // Wait for certificate
        let final_order = self.wait_for_order(&order.id).await?;

        // Download certificate
        let cert_url = final_order.certificate.ok_or(TlsError::NoCertificateInOrder)?;
        let cert_pem: Vec<u8> = self.get_acme(&cert_url, &account.key).await?
            .text().await?
            .into_bytes().collect();

        Ok(Certificate {
            cert_pem,
            key_pem: vec![],
            issuer: "Let's Encrypt".to_string(),
            not_before: chrono::Utc::now(),
            not_after: chrono::Utc::now() + chrono::Duration::days(90),
            subjects: request.sans,
        })
    }

    async fn revoke(&self, _ctx: &TlsContext, _cert: &Certificate) -> Result<(), TlsError> {
        // Implement revocation
        Ok(())
    }
}
```

### 4.3 Certificate Storage

```rust
/// Certificate storage trait
#[async_trait]
pub trait CertificateStorage: Send + Sync {
    async fn store(&self, key: &str, data: &[u8]) -> Result<(), StorageError>;
    async fn load(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError>;
}

/// File system storage implementation
pub struct FileSystemStorage {
    base_path: PathBuf,
}

#[async_trait]
impl CertificateStorage for FileSystemStorage {
    async fn store(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        let path = self.base_path.join(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Atomic write
        let tmp_path = path.with_extension("tmp");
        tokio::fs::write(&tmp_path, data).await?;
        tokio::fs::rename(&tmp_path, &path).await?;

        Ok(())
    }

    async fn load(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let path = self.base_path.join(key);
        match tokio::fs::read(path).await {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ... other methods
}
```

---

## 5. HTTP Server Translation

### 5.1 Server Structure

```rust
use http::{Request, Response};
use std::net::SocketAddr;

/// HTTP Server configuration
#[derive(Debug)]
pub struct HttpServerConfig {
    pub listen: Vec<String>,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub idle_timeout: Duration,
    pub tls_config: Option<TlsConfig>,
}

/// HTTP Server
pub struct HttpServer {
    config: HttpServerConfig,
    routes: RouteList,
    tls_manager: Option<Arc<TlsManager>>,
}

impl HttpServer {
    pub fn new(config: HttpServerConfig) -> Self {
        Self {
            config,
            routes: RouteList::new(),
            tls_manager: None,
        }
    }

    pub async fn start(&self) -> Result<(), ServerError> {
        for addr in &self.config.listen {
            let listener = self.bind(addr).await?;

            // Clone for each connection
            let routes = self.routes.clone();
            let tls_manager = self.tls_manager.clone();

            tokio::spawn(async move {
                loop {
                    let (stream, peer_addr) = listener.accept().await?;

                    // Handle connection
                    let routes = routes.clone();
                    tokio::spawn(async move {
                        if let Some(ref tls) = tls_manager {
                            // TLS connection
                            Self::handle_tls_connection(stream, peer_addr, routes, tls).await
                        } else {
                            // Plain HTTP
                            Self::handle_connection(stream, peer_addr, routes).await
                        }
                    });
                }
            });
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        peer_addr: SocketAddr,
        routes: RouteList,
    ) -> Result<(), ServerError> {
        // Use hyper for HTTP parsing
        let io = TokioIo::new(stream);

        let service = hyper::service::service_fn(move |req| {
            let routes = routes.clone();
            async move {
                routes.serve_request(req).await
            }
        });

        hyper::server::conn::http1::Builder::new()
            .serve_connection(io, service)
            .await?;

        Ok(())
    }
}
```

### 5.2 Route Matching

```rust
/// Route with matchers and handlers
pub struct Route {
    pub matchers: Vec<Arc<dyn HttpMatcher>>,
    pub handlers: Vec<Arc<dyn HttpHandler>>,
    pub terminal: bool,
}

/// Route list (compiled middleware chain)
#[derive(Clone)]
pub struct RouteList {
    routes: Arc<Vec<Route>>,
}

impl RouteList {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(Vec::new()),
        }
    }

    pub fn add(&mut self, route: Route) {
        Arc::make_mut(&mut self.routes).push(route);
    }

    pub async fn serve_request(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, HandlerError> {
        // Find matching route
        for route in self.routes.iter() {
            // Check all matchers (AND logic)
            let matches = route.matchers.iter().all(|m| m.matches(&req));

            if matches {
                // Execute handlers in chain
                return self.execute_handlers(req, &route.handlers).await;
            }
        }

        // No match - 404
        Err(HandlerError::NotFound)
    }

    async fn execute_handlers(
        &self,
        req: Request<Body>,
        handlers: &[Arc<dyn HttpHandler>],
    ) -> Result<Response<Body>, HandlerError> {
        // Build handler chain
        let mut next = Next { handler: None };

        // Build chain in reverse order
        for handler in handlers.iter().rev() {
            next = Next {
                handler: Some(handler.clone()),
            };
        }

        // Execute first handler (which calls next, etc.)
        if let Some(first) = handlers.first() {
            first.serve_http(req, next).await
        } else {
            Err(HandlerError::NotFound)
        }
    }
}
```

---

## 6. Reverse Proxy Translation

### 6.1 Transport Trait

```rust
/// Transport for backend communication
pub trait Transport: Send + Sync {
    fn send(
        &self,
        req: Request<Body>,
    ) -> impl Future<Output = Result<Response<Body>, HandlerError>> + Send;
}

/// HTTP Transport implementation
#[derive(Debug)]
pub struct HttpTransport {
    config: HttpTransportConfig,
    client: reqwest::Client,
}

#[derive(Debug)]
pub struct HttpTransportConfig {
    pub dial_timeout: Duration,
    pub keep_alive: Duration,
    pub max_conns_per_host: usize,
    pub max_idle_conns: usize,
    pub idle_conn_timeout: Duration,
    pub tls_config: Option<TlsConfig>,
}

impl Default for HttpTransportConfig {
    fn default() -> Self {
        Self {
            dial_timeout: Duration::from_secs(30),
            keep_alive: Duration::from_secs(300),
            max_conns_per_host: 0,  // Unlimited
            max_idle_conns: 100,
            idle_conn_timeout: Duration::from_secs(90),
            tls_config: None,
        }
    }
}

impl HttpTransport {
    pub fn new(config: HttpTransportConfig) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(config.dial_timeout)
            .timeout(config.idle_conn_timeout)
            .pool_max_idle_per_host(config.max_idle_conns)
            .pool_idle_timeout(config.idle_conn_timeout)
            .build()
            .unwrap();

        Self { config, client }
    }
}

impl Transport for HttpTransport {
    async fn send(&self, req: Request<Body>) -> Result<Response<Body>, HandlerError> {
        // Convert hyper::Request to reqwest::Request
        let method = req.method().clone();
        let url = req.uri().to_string();
        let headers = req.headers().clone();
        let body = req.into_body();

        let mut req_builder = self.client.request(method, &url);
        req_builder = req_builder.headers(headers);
        req_builder = req_builder.body(body.to_vec());

        let response = req_builder.send().await
            .map_err(|e| HandlerError::UpstreamError(e.to_string()))?;

        // Convert back to hyper response
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.bytes().await
            .map_err(|e| HandlerError::UpstreamError(e.to_string()))?;

        let mut response = Response::builder()
            .status(status)
            .body(Body::from(body))
            .unwrap();

        *response.headers_mut() = headers;

        Ok(response)
    }
}
```

---

## 7. Configuration Parsing

### 7.1 Caddyfile-like Parser

```rust
// See 04-caddyfile-config-deep-dive.md for full implementation

pub struct CaddyfileParser;

impl CaddyfileParser {
    pub fn parse(input: &str) -> Result<Vec<ServerBlock>, ParseError> {
        // 1. Expand environment variables
        let expanded = expand_env_vars(input);

        // 2. Tokenize
        let tokens = tokenize(&expanded, "Caddyfile")?;

        // 3. Parse server blocks
        let mut parser = Parser::new(tokens, "Caddyfile");
        parser.parse_all()
    }
}

#[derive(Debug)]
pub struct ServerBlock {
    pub keys: Vec<String>,
    pub segments: Vec<Segment>,
}

#[derive(Debug)]
pub struct Segment {
    pub directive: String,
    pub args: Vec<String>,
    pub body: Option<Vec<Segment>>,
}
```

### 7.2 JSON Configuration

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CaddyConfig {
    pub admin: Option<AdminConfig>,
    pub logging: Option<LoggingConfig>,
    pub storage: Option<StorageConfig>,
    pub apps: AppsConfig,
}

#[derive(Debug, Deserialize)]
pub struct AppsConfig {
    pub http: Option<HttpAppConfig>,
    pub tls: Option<TlsAppConfig>,
}

#[derive(Debug, Deserialize)]
pub struct HttpAppConfig {
    pub servers: HashMap<String, ServerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub listen: Vec<String>,
    pub routes: Option<Vec<RouteConfig>>,
    pub tls_connection_policies: Option<Vec<TlsPolicyConfig>>,
}

#[derive(Debug, Deserialize)]
pub struct RouteConfig {
    #[serde(rename = "match")]
    pub matchers: Option<Vec<MatcherConfig>>,
    #[serde(rename = "handle")]
    pub handlers: Option<Vec<HandlerConfig>>,
    pub terminal: Option<bool>,
}

// Usage
let config: CaddyConfig = serde_json::from_str(json_str)?;
```

---

## 8. Valtron Integration

### 8.1 TaskIterator for HTTP Handling

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};

/// HTTP request handling task (no async/await)
pub struct HandleHttpRequest {
    request: Option<Request<Body>>,
    routes: Arc<RouteList>,
    response_tx: Option<mpsc::Sender<Result<Response<Body>, HandlerError>>>,
}

impl HandleHttpRequest {
    pub fn new(
        request: Request<Body>,
        routes: Arc<RouteList>,
        response_tx: mpsc::Sender<Result<Response<Body>, HandlerError>>,
    ) -> Self {
        Self {
            request: Some(request),
            routes,
            response_tx: Some(response_tx),
        }
    }
}

impl TaskIterator for HandleHttpRequest {
    type Ready = Result<Response<Body>, HandlerError>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        // Take the request
        let request = self.request.take()?;

        // Process synchronously (or use blocking HTTP client)
        let response = futures::executor::block_on(
            self.routes.serve_request(request)
        );

        // Send response
        if let Some(tx) = self.response_tx.take() {
            let _ = tx.blocking_send(response.clone());
        }

        Some(TaskStatus::Ready(response))
    }
}
```

### 8.2 ACME Challenge Task

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};

/// ACME challenge solving task
pub struct SolveAcmeChallenge {
    challenge_url: String,
    account_key: Arc<P256PrivateKey>,
    status: ChallengeStatus,
}

enum ChallengeStatus {
    Pending,
    Waiting { poll_url: String, attempts: u32 },
    Complete { result: Result<(), TlsError> },
}

impl TaskIterator for SolveAcmeChallenge {
    type Ready = Result<(), TlsError>;
    type Pending = Duration;  // Wait time before next poll
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, NoSpawner>> {
        match &mut self.status {
            ChallengeStatus::Pending => {
                // Send challenge notification to ACME server
                // This would use blocking HTTP client
                let response = blocking_get(&self.challenge_url);

                self.status = ChallengeStatus::Waiting {
                    poll_url: extract_poll_url(&response),
                    attempts: 0,
                };

                Some(TaskStatus::Pending(Duration::from_secs(1)))
            }

            ChallengeStatus::Waiting { poll_url, attempts } => {
                *attempts += 1;

                if *attempts > 30 {
                    self.status = ChallengeStatus::Complete {
                        result: Err(TlsError::ChallengeTimeout),
                    };
                    return self.next();
                }

                // Poll challenge status
                let response = blocking_get(poll_url);
                let status = parse_challenge_status(&response);

                match status {
                    "valid" => {
                        self.status = ChallengeStatus::Complete {
                            result: Ok(()),
                        };
                    }
                    "invalid" | "expired" => {
                        self.status = ChallengeStatus::Complete {
                            result: Err(TlsError::ChallengeFailed),
                        };
                    }
                    _ => {
                        // Still pending, wait and poll again
                        return Some(TaskStatus::Pending(Duration::from_secs(2)));
                    }
                }

                self.next()
            }

            ChallengeStatus::Complete { result } => {
                Some(TaskStatus::Ready(result.clone()))
            }
        }
    }
}
```

### 8.3 Single-Threaded Executor Usage

```rust
use foundation_core::valtron::single::{initialize, spawn, run_until_complete};

fn main() {
    // Initialize single-threaded executor
    initialize(42);  // Seed for deterministic behavior

    // Spawn HTTP server task
    spawn()
        .with_task(HttpServerTask::new(config))
        .with_resolver(Box::new(FnReady::new(|result, _| {
            if let Err(e) = result {
                eprintln!("Server error: {}", e);
            }
        })))
        .schedule()
        .expect("Failed to schedule server task");

    // Run until complete
    run_until_complete();
}
```

### 8.4 Lambda Deployment

```rust
// For AWS Lambda, use valtron without tokio runtime
// Entry point for Lambda

use aws_lambda_events::event::alb::AlbTargetGroupRequest;
use aws_lambda_events::response::AlbTargetGroupResponse;
use lambda_runtime::{service_fn, Error, LambdaEvent};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize server once (cold start)
    let routes = Arc::new(RouteList::new());
    // ... configure routes ...

    // Lambda handler
    let func = service_fn(move |event: LambdaEvent<AlbTargetGroupRequest>| {
        let routes = routes.clone();

        async move {
            // Convert ALB event to HTTP request
            let request = event_to_request(event.payload)?;

            // Process with valtron executor
            let response = tokio::task::spawn_blocking(move || {
                // Use blocking executor for valtron
                foundation_core::valtron::single::initialize(42);

                // Create and run task
                let (tx, rx) = std::sync::mpsc::channel();
                foundation_core::valtron::single::spawn()
                    .with_task(HandleHttpRequest::new(request, routes, tx))
                    .schedule()
                    .unwrap();

                foundation_core::valtron::single::run_until_complete();

                rx.recv().unwrap()
            }).await.unwrap()?;

            // Convert back to ALB response
            Ok(response_to_alb(response))
        }
    });

    lambda_runtime::run(func).await
}
```

---

## Summary

### Key Translations

| Go Concept | Rust Equivalent |
|------------|----------------|
| `interface{}` | `dyn Trait` |
| `json.RawMessage` | `serde_json::Value` |
| `context.Context` | Custom `Context` struct |
| Goroutines | `tokio::spawn` or valtron |
| `sync.Mutex` | `tokio::sync::RwLock` |
| `http.RoundTripper` | `dyn Transport` trait |
| `certmagic.Issuer` | `dyn CertificateIssuer` trait |

### Valtron Patterns

- Use `TaskIterator` for async-like operations without async/await
- `Ready` type is the final result
- `Pending` type is the wait indicator
- Use blocking HTTP clients with valtron

### For ewe_platform

1. Module system with trait objects
2. ACME implementation with `reqwest` (blocking for Lambda)
3. HTTP handling with valtron TaskIterator
4. Configuration with serde_json

---

*Continue with [Production-Grade](production-grade.md) for deployment details.*
