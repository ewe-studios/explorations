# Rust mTLS Implementation Guide

## Overview

This document provides comprehensive Rust code examples for implementing mTLS in production services using `rustls`, `tokio-rustls`, `hyper`, `axum`, and `tonic`.

## Core TLS Configuration

### Loading Certificates

```rust
use rustls::{Certificate, PrivateKey, ServerConfig, ClientConfig};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::fs::File;
use std::io::{self, BufReader};
use std::sync::Arc;
use std::path::Path;

/// Load certificates from a PEM file
fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    let cert_file = File::open(path)?;
    let mut reader = BufReader::new(cert_file);
    certs(&mut reader)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid certificate"))
        .map(|certs| certs.into_iter().map(Certificate).collect())
}

/// Load private key from PEM file
fn load_private_key(path: &Path) -> io::Result<PrivateKey> {
    let key_file = File::open(path)?;
    let mut reader = BufReader::new(key_file);

    // Try PKCS#8 format first
    let mut keys = pkcs8_private_keys(&mut reader)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))?;

    // Fall back to RSA format
    if keys.is_empty() {
        let key_file = File::open(path)?;
        let mut reader = BufReader::new(key_file);
        keys = rsa_private_keys(&mut reader)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))?;
    }

    keys.into_iter()
        .next()
        .map(PrivateKey)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no private key found"))
}

/// Load CA certificates for trust store
fn load_root_certs(path: &Path) -> io::Result<rustls::RootCertStore> {
    let mut root_store = rustls::RootCertStore::empty();

    let certs = load_certs(path)?;
    let (added, ignored) = root_store.add_parsable_certificates(&certs);

    println!("Added {} root certificates, ignored {}", added, ignored);

    Ok(root_store)
}
```

### TLS Configuration Builder

```rust
use rustls::ServerConfig;
use rustls::server::{ServerConnection, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use std::sync::Arc;

pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
    pub ca_path: String,
    pub client_auth_required: bool,
}

impl TlsConfig {
    /// Build server TLS configuration
    pub fn build_server_config(&self) -> io::Result<Arc<ServerConfig>> {
        let certs = load_certs(Path::new(&self.cert_path))?;
        let key = load_private_key(Path::new(&self.key_path))?;

        let mut config = ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(
                if self.client_auth_required {
                    // Require client certificates
                    AllowAnyAuthenticatedClient::new(load_root_certs(Path::new(&self.ca_path))?)
                } else {
                    // Optional client certificates
                    AllowAnyAnonymousOrAuthenticatedClient::new(
                        load_root_certs(Path::new(&self.ca_path))?
                    )
                }
            )
            .with_single_cert(certs, key)?;

        // Configure ALPN for HTTP/2 support
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(Arc::new(config))
    }

    /// Build client TLS configuration
    pub fn build_client_config(&self) -> io::Result<Arc<ClientConfig>> {
        let certs = load_certs(Path::new(&self.cert_path))?;
        let key = load_private_key(Path::new(&self.key_path))?;
        let root_store = load_root_certs(Path::new(&self.ca_path))?;

        let mut config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_client_auth_cert(certs, key)?;

        // Configure ALPN
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(Arc::new(config))
    }
}
```

## mTLS HTTP Server (Axum)

### Basic mTLS Server

```rust
use axum::{Router, routing::get, extract::State};
use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    // Application state
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load TLS configuration
    let rustls_config = RustlsConfig::from_pem_file(
        "certs/server.crt",
        "keys/server.key",
    )
    .await?;

    // Configure client certificate verification
    let rustls_config = rustls_config
        .with_client_certificate_verifier(
            axum_server::tls_rustls::client_cert_verifier(
                "certs/ca.crt".into()
            )
        );

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/data", get(data_handler))
        .with_state(AppState {});

    let addr = SocketAddr::from(([0, 0, 0, 0], 8443));

    println!("Starting mTLS server on {}", addr);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn data_handler() -> &'static str {
    "Protected data"
}
```

### Extracting Client Certificate Information

```rust
use axum::{
    extract::ConnectInfo,
    http::request::Parts,
    RequestPartsExt,
};
use axum_server::tls_rustls::RustlsConnection;
use rustls::server::TlsExtensions;
use std::net::SocketAddr;

/// Extract client certificate subject from TLS connection
fn extract_client_cert(conn: &RustlsConnection) -> Option<String> {
    conn.peer_certificates()
        .and_then(|certs| certs.first())
        .map(|cert| {
            // Parse certificate to extract subject
            use x509_parser::prelude::*;
            parse_x509_certificate(&cert.0)
                .ok()
                .map(|(_, cert)| cert.tbs_certificate.subject.to_string())
                .unwrap_or_default()
        })
}

/// Custom extractor for client certificate info
struct ClientCertInfo {
    subject: String,
    san_dns: Vec<String>,
    san_uri: Vec<String>,
    serial: String,
}

impl ClientCertInfo {
    fn from_connection(conn: &RustlsConnection) -> Option<Self> {
        conn.peer_certificates()
            .and_then(|certs| certs.first())
            .and_then(|cert| {
                use x509_parser::prelude::*;
                let (_, parsed) = parse_x509_certificate(&cert.0).ok()?;

                let subject = parsed.tbs_certificate.subject.to_string();

                // Extract SANs
                let mut san_dns = Vec::new();
                let mut san_uri = Vec::new();

                for ext in &parsed.tbs_certificate.extensions {
                    if let ExtensionKind::SubjectAlternativeName(san) = ext.parsed_extension() {
                        for name in &san.general_names {
                            match name {
                                GeneralName::DNSName(dns) => san_dns.push(dns.to_string()),
                                GeneralName::URI(uri) => san_uri.push(uri.to_string()),
                                _ => {}
                            }
                        }
                    }
                }

                let serial = format!("{:x}", parsed.tbs_certificate.serial);

                Some(ClientCertInfo {
                    subject,
                    san_dns,
                    san_uri,
                    serial,
                })
            })
    }
}
```

### Authorization Based on Client Certificate

```rust
use axum::{
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use std::collections::HashSet;

/// Middleware to validate client certificate SPIFFE ID
async fn validate_spiffe_id(
    cert_info: ClientCertInfo,
    next: Next,
) -> Result<Response, StatusCode> {
    // Expected SPIFFE IDs for this service
    let allowed_ids = HashSet::from([
        "spiffe://example.com/ns/default/sa/api-gateway",
        "spiffe://example.com/ns/default/sa/admin-service",
    ]);

    // Check if any SAN URI matches allowed SPIFFE IDs
    let is_authorized = cert_info.san_uri.iter()
        .any(|uri| allowed_ids.contains(uri.as_str()));

    if is_authorized {
        Ok(next.run(Request::new(cert_info)).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Build router with mTLS authorization
fn build_secure_router() -> Router {
    Router::new()
        .route("/admin", get(admin_handler))
        .route("/data", get(data_handler))
        .layer(middleware::from_fn(validate_spiffe_id))
}
```

## mTLS HTTP Client (Hyper)

### Basic mTLS Client

```rust
use hyper::{Body, Client, Request, Response, Method};
use hyper_rustls::HttpsConnector;
use rustls::ClientConfig;
use std::sync::Arc;

pub struct MtlsClient {
    client: Client<HttpsConnector<hyper::client::HttpConnector>, Body>,
}

impl MtlsClient {
    /// Create new mTLS client
    pub fn new(cert_path: &str, key_path: &str, ca_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let tls_config = TlsConfig {
            cert_path: cert_path.to_string(),
            key_path: key_path.to_string(),
            ca_path: ca_path.to_string(),
            client_auth_required: true,
        };

        let client_config = tls_config.build_client_config()?;

        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(client_config)
            .https_or_http()
            .enable_http2()
            .build();

        let client = Client::builder()
            .build(connector);

        Ok(MtlsClient { client })
    }

    /// Make GET request
    pub async fn get(&self, url: &str) -> Result<Response<Body>, hyper::Error> {
        let req = Request::builder()
            .method(Method::GET)
            .uri(url)
            .body(Body::empty())?;

        self.client.request(req).await
    }

    /// Make POST request with body
    pub async fn post(&self, url: &str, body: Body) -> Result<Response<Body>, hyper::Error> {
        let req = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header("content-type", "application/json")
            .body(body)?;

        self.client.request(req).await
    }
}
```

### Client with Retry Logic

```rust
use std::time::Duration;
use tokio::time::sleep;

impl MtlsClient {
    /// Make request with retry logic
    pub async fn get_with_retry(
        &self,
        url: &str,
        max_retries: u32,
        backoff_ms: u64,
    ) -> Result<Response<Body>, Box<dyn std::error::Error>> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            match self.get(url).await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    } else {
                        last_error = Some(format!("HTTP error: {}", response.status()));
                    }
                }
                Err(e) => {
                    last_error = Some(format!("Request failed: {}", e));
                }
            }

            if attempt < max_retries - 1 {
                sleep(Duration::from_millis(backoff_ms * (1 << attempt))).await;
            }
        }

        Err(last_error.unwrap_or("Unknown error".into()))
    }
}
```

## mTLS gRPC Server (Tonic)

### gRPC Server with mTLS

```rust
use tonic::{transport::Server, Request, Response, Status};
use tonic_examples::my_service_server::{MyService, MyServiceServer};
use tokio_stream::StreamExt;
use std::net::SocketAddr;

pub struct MyServiceImpl;

#[tonic::async_trait]
impl MyService for MyServiceImpl {
    async fn my_method(
        &self,
        request: Request<MyRequest>,
    ) -> Result<Response<MyResponse>, Status> {
        // Extract client certificate info from metadata
        let client_cert = request.metadata()
            .get("x-client-cert-subject")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        println!("Request from client: {}", client_cert);

        Ok(Response::new(MyResponse {
            message: format!("Hello, authenticated client!"),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "[::]:50051".parse()?;

    // Load TLS config
    let tls_config = TlsConfig {
        cert_path: "certs/server.crt".to_string(),
        key_path: "keys/server.key".to_string(),
        ca_path: "certs/ca.crt".to_string(),
        client_auth_required: true,
    };

    let server_config = tls_config.build_server_config()?;

    let identity = rustls::server::ResolvesServerCertUsingSni::new();

    Server::builder()
        .tls_config(tonic::transport::ServerTlsConfig::new()
            .rustls_server_config(server_config)
        )?
        .add_service(MyServiceServer::new(MyServiceImpl))
        .serve(addr)
        .await?;

    Ok(())
}
```

### gRPC Client with mTLS

```rust
use tonic::transport::{Channel, ClientTlsConfig};
use tonic_examples::my_service_client::MyServiceClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tls_config = TlsConfig {
        cert_path: "certs/client.crt".to_string(),
        key_path: "keys/client.key".to_string(),
        ca_path: "certs/ca.crt".to_string(),
        client_auth_required: true,
    };

    let client_tls_config = tls_config.build_client_config()?;

    let channel = Channel::from_static("https://localhost:50051")
        .tls_config(ClientTlsConfig::new()
            .rustls_client_config(client_tls_config)
        )?
        .connect()
        .await?;

    let mut client = MyServiceClient::new(channel);

    let response = client.my_method(MyRequest {
        // request fields
    }).await?;

    println!("Response: {:?}", response.get_ref());

    Ok(())
}
```

## WebSocket mTLS

### WebSocket Server with mTLS

```rust
use axum::{
    extract::ws::{WebSocketUpgrade, WebSocket},
    response::IntoResponse,
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;
use futures::{sink::SinkExt, stream::StreamExt};

async fn ws_handler(
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.next().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                println!("Received: {}", text);
                socket.send(axum::extract::ws::Message::Text(text)).await.ok();
            }
            _ => break,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rustls_config = RustlsConfig::from_pem_file(
        "certs/server.crt",
        "keys/server.key",
    )
    .await?;

    let app = Router::new()
        .route("/ws", get(ws_handler));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8443));

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

## Certificate Rotation

### Hot Reload Server Certificates

```rust
use tokio::sync::watch;
use std::sync::Arc;

pub struct CertReloader {
    config_tx: watch::Sender<Arc<ServerConfig>>,
}

impl CertReloader {
    pub fn new(initial_config: Arc<ServerConfig>) -> Self {
        let (tx, _rx) = watch::channel(initial_config);
        CertReloader { config_tx: tx }
    }

    /// Reload certificates from disk
    pub fn reload(&self, cert_path: &str, key_path: &str) -> Result<(), io::Error> {
        let certs = load_certs(Path::new(cert_path))?;
        let key = load_private_key(Path::new(key_path))?;

        // Get current config and update certificates
        let current = self.config_tx.borrow();
        let mut new_config = (*current).clone();

        // Note: rustls doesn't support hot reload of certs directly
        // You need to create a new ServerConfig and update the server
        // This is a simplified example

        Ok(())
    }

    pub fn get_current(&self) -> Arc<ServerConfig> {
        self.config_tx.borrow().clone()
    }
}

// For true hot reload, consider using a proxy pattern or connection pool
// that can swap out the TLS config between connections
```

### Graceful Certificate Rotation

```rust
use tokio::task::JoinHandle;
use std::time::Duration;

pub struct GracefulCertRotation {
    reloader: CertReloader,
    shutdown: tokio::sync::Notify,
}

impl GracefulCertRotation {
    pub fn spawn_rotation_task(
        cert_path: String,
        key_path: String,
        check_interval: Duration,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);

            loop {
                interval.tick().await;

                // Check if certificates need rotation
                if let Ok(expiry) = get_cert_expiry(&cert_path) {
                    let days_left = (expiry - chrono::Utc::now()).num_days();

                    if days_left < 7 {
                        println!("Certificates expiring in {} days, triggering rotation", days_left);
                        // Trigger rotation (via file watch, API call, etc.)
                    }
                }
            }
        })
    }
}

fn get_cert_expiry(cert_path: &str) -> Result<chrono::DateTime<chrono::Utc>, Box<dyn std::error::Error>> {
    use x509_parser::prelude::*;
    use std::fs;

    let cert_pem = fs::read(cert_path)?;
    let (_, cert) = parse_x509_certificate(&cert_pem)?;

    Ok(cert.validity().not_after)
}
```

## Testing mTLS

### Integration Test Setup

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rcgen::{Certificate, CertificateParams};
    use std::net::TcpListener;

    fn generate_test_certs() -> (String, String, String) {
        // Generate CA
        let mut ca_params = CertificateParams::default();
        ca_params.is_ca = Some(rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained));
        let ca_cert = Certificate::from_params(ca_params).unwrap();
        let ca_pem = ca_cert.serialize_pem().unwrap();

        // Generate server cert
        let mut server_params = CertificateParams::default();
        server_params.subject_alt_names = vec![
            rcgen::SanType::DnsName("localhost".to_string()),
        ];
        let server_cert = Certificate::from_params(server_params).unwrap();
        let server_cert_pem = server_cert.serialize_pem().unwrap();
        let server_key_pem = server_cert.serialize_private_key_pem();

        // Write to temp files
        let temp_dir = std::env::temp_dir();

        let ca_path = temp_dir.join("test-ca.pem");
        let server_cert_path = temp_dir.join("test-server.pem");
        let server_key_path = temp_dir.join("test-server-key.pem");

        std::fs::write(&ca_path, &ca_pem).unwrap();
        std::fs::write(&server_cert_path, &server_cert_pem).unwrap();
        std::fs::write(&server_key_path, &server_key_pem).unwrap();

        (
            ca_path.to_string_lossy().to_string(),
            server_cert_path.to_string_lossy().to_string(),
            server_key_path.to_string_lossy().to_string(),
        )
    }

    #[tokio::test]
    async fn test_mtls_server() {
        let (ca_path, cert_path, key_path) = generate_test_certs();

        let tls_config = TlsConfig {
            cert_path,
            key_path,
            ca_path,
            client_auth_required: true,
        };

        let server_config = tls_config.build_server_config().unwrap();

        // Assert server config is valid
        assert!(!server_config.alpn_protocols.is_empty());
    }
}
```

### Test Utilities

```rust
/// Test helper to create mTLS client for testing
pub fn create_test_mtls_client(
    cert_path: &str,
    key_path: &str,
    ca_path: &str,
) -> MtlsClient {
    MtlsClient::new(cert_path, key_path, ca_path)
        .expect("Failed to create mTLS client")
}

/// Assert that connection fails without valid client cert
#[tokio::test]
async fn test_rejects_invalid_client_cert() {
    // Start server with client auth required

    // Try to connect without client cert
    // Should fail with TLS handshake error

    // Try to connect with wrong CA cert
    // Should fail with certificate verification error
}
```

## Performance Considerations

### Connection Pooling

```rust
use hyper::client::connect::dns::GaiResolver;
use hyper::client::HttpConnector;

/// Create client with connection pooling
pub fn create_pooled_client(
    pool_max_idle: usize,
    pool_timeout: Duration,
) -> Result<MtlsClient, Box<dyn std::error::Error>> {
    let tls_config = TlsConfig::from_env()?;
    let client_config = tls_config.build_client_config()?;

    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(client_config)
        .https_or_http()
        .enable_http2()
        .build();

    let client = Client::builder()
        .pool_max_idle_per_host(pool_max_idle)
        .pool_idle_timeout(pool_timeout)
        .http2_keep_alive_interval(Duration::from_secs(30))
        .http2_keep_alive_timeout(Duration::from_secs(10))
        .build(connector);

    Ok(MtlsClient { client })
}
```

### Session Resumption

```rust
// rustls enables session resumption by default
// For additional control:

use rustls::ServerConfig;
use rustls::server::ServerSessionMemoryCache;

let mut config = ServerConfig::builder()
    .with_safe_defaults()
    .with_client_cert_verifier(verifier)
    .with_cert(certified_key);

// Configure session cache
config.session_storage = ServerSessionMemoryCache::new(256);

// Session tickets for TLS 1.3
config.ticketer = rustls::Ticketer::new().unwrap();
```

## Common Patterns

### Service-to-Service Client Wrapper

```rust
pub struct ServiceClient {
    client: MtlsClient,
    base_url: String,
}

impl ServiceClient {
    pub fn new(target_service: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let client = MtlsClient::new(
            &std::env::var("MTLS_CERT_PATH")?,
            &std::env::var("MTLS_KEY_PATH")?,
            &std::env::var("MTLS_CA_PATH")?,
        )?;

        Ok(ServiceClient {
            client,
            base_url: format!("https://{}", target_service),
        })
    }

    pub async fn call(&self, path: &str, method: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}{}", self.base_url, path);
        let response = match method {
            "GET" => self.client.get(&url).await?,
            "POST" => self.client.post(&url, Body::empty()).await?,
            _ => return Err("Unsupported method".into()),
        };

        let body = hyper::body::to_bytes(response.into_body()).await?;
        Ok(String::from_utf8_lossy(&body).to_string())
    }
}
```

### Middleware for Logging

```rust
use tower::{Service, ServiceExt};
use tower_http::trace::TraceLayer;

fn build_app_with_logging() -> Router {
    Router::new()
        .route("/api/data", get(data_handler))
        .layer(TraceLayer::new_for_http())
}
```
