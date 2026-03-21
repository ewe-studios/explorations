---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/rust-mtls/exploration.md
repository: N/A - Greenfield implementation
revised_at: 2026-03-21
workspace: rust-mtls-workspace
---

# Rust Revision: Mutual mTLS Production Implementation

## Overview

This document provides a complete, production-ready implementation guide for mutual TLS (mTLS) in Rust. The implementation uses `rustls` for TLS (pure Rust, no OpenSSL dependency), `axum` for the web framework, and `tokio` for async runtime.

Key features covered:
- Full mTLS server with client certificate verification
- mTLS HTTP client
- Certificate generation utilities for testing
- Certificate validation and identity extraction
- Automated certificate rotation patterns
- Production deployment configurations

## Workspace Structure

```
rust-mtls-workspace/
├── Cargo.toml                          # Workspace root
├── Cargo.lock
├── rust-revision.md
├── crates/
│   ├── mtls-core/                      # Core mTLS primitives
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs               # TLS configuration types
│   │       ├── identity.rs             # Certificate identity extraction
│   │       ├── verifier.rs             # Client certificate verification
│   │       └── error.rs                # Error types
│   ├── mtls-server/                    # mTLS server implementation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs               # Main server setup
│   │       ├── middleware.rs           # Authentication middleware
│   │       └── handlers.rs             # Request handlers
│   ├── mtls-client/                    # mTLS HTTP client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs               # HTTP client with mTLS
│   │       └── cert_manager.rs         # Client cert management
│   ├── mtls-certs/                     # Certificate utilities
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── generator.rs            # Test cert generation
│   │       ├── loader.rs               # Cert/key loading
│   │       ├── store.rs                # Certificate storage
│   │       └── rotation.rs             # Auto-rotation logic
│   └── mtls-examples/                  # Example binary
│       ├── Cargo.toml
│       └── src/
│           ├── bin/
│           │   ├── server.rs           # Example mTLS server
│           │   ├── client.rs           # Example mTLS client
│           │   └── certgen.rs          # Certificate generator tool
├── tests/
│   └── integration/
│       ├── Cargo.toml
│       └── src/
│           └── main.rs                 # Integration tests
└── deploy/
    ├── kubernetes/
    │   ├── server-deployment.yaml
    │   ├── server-service.yaml
    │   └── cert-secret.yaml
    └── docker/
        └── Dockerfile
```

### Crate Breakdown

#### mtls-core

- **Purpose:** Core mTLS primitives, configuration, and verification
- **Type:** library
- **Public API:** `MtlsConfig`, `ClientCertVerifier`, `ClientIdentity`, `MtlsError`
- **Dependencies:** tokio, rustls, x509-parser, thiserror, tracing

#### mtls-server

- **Purpose:** mTLS-enabled HTTP server using axum
- **Type:** library
- **Public API:** `MtlsServer`, `MtlsRouter`, `RequireClientCert` middleware
- **Dependencies:** axum, axum-server, tokio, rustls, mtls-core, tower

#### mtls-client

- **Purpose:** mTLS HTTP client for making authenticated requests
- **Type:** library
- **Public API:** `MtlsClient`, `ClientCertConfig`, `MtlsRequestBuilder`
- **Dependencies:** reqwest, tokio, rustls, mtls-core

#### mtls-certs

- **Purpose:** Certificate generation, loading, storage, and rotation
- **Type:** library
- **Public API:** `CertGenerator`, `CertLoader`, `CertStore`, `CertRotator`
- **Dependencies:** rcgen, tokio, rustls-pemfile, x509-parser, thiserror

#### mtls-examples

- **Purpose:** Example binaries demonstrating mTLS usage
- **Type:** binary
- **Dependencies:** All workspace crates, clap

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Async runtime | tokio | 1.0 (full) | Complete async runtime with all features |
| TLS library | rustls | 0.23+ | Pure Rust TLS, memory safe |
| TLS integration | tokio-rustls | 0.26+ | Async TLS streams |
| Web framework | axum | 0.8+ | Ergonomic, type-safe web framework |
| TLS server | axum-server | 0.7+ | AXUM with TLS support |
| HTTP client | reqwest | 0.12+ | Async HTTP client with rustls |
| Certificate parsing | x509-parser | 0.17+ | X.509 certificate parsing |
| Certificate generation | rcgen | 0.13+ | Test certificate generation |
| PEM parsing | rustls-pemfile | 2.0+ | PEM file parsing |
| System certs | rustls-native-certs | 0.8+ | Load OS CA certificates |
| Web PKI | webpki-roots | 0.26+ | Mozilla's root certs |
| Error handling | thiserror | 2.0 | Derive macros for error types |
| Logging | tracing | 0.1 | Async-aware logging |
| Serialization | serde + serde_json | 1.0 | JSON serialization |
| CLI parsing | clap | 4.0 | CLI argument parsing |
| Tower middleware | tower + tower-http | 0.5 | Service middleware |
| Time handling | chrono | 0.4 | Time and date handling |
| Bytes | bytes | 1.0 | Byte buffer handling |
| PIN types | pin-project-lite | 0.2 | Pinning for async |

## Type System Design

### Core Types

```rust
// ============ CONFIGURATION ============

/// mTLS configuration for server or client
#[derive(Debug, Clone)]
pub struct MtlsConfig {
    /// Path to server certificate
    pub server_cert_path: PathBuf,
    /// Path to server private key
    pub server_key_path: PathBuf,
    /// Path to CA certificate for verifying clients
    pub ca_cert_path: PathBuf,
    /// Whether client certificates are required
    pub client_auth: ClientAuthMode,
    /// TLS versions to allow
    pub tls_versions: Vec<TlsVersion>,
    /// Cipher suites to allow
    pub cipher_suites: Vec<CipherSuite>,
}

/// Client certificate authentication mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientAuthMode {
    /// Client certificate is required
    Required,
    /// Client certificate is optional
    Optional,
    /// No client certificate requested
    None,
}

/// TLS version enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

// ============ IDENTITY ============

/// Extracted client identity from certificate
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientIdentity {
    /// Common Name (CN) from certificate subject
    pub common_name: String,
    /// Organization (O) from certificate subject
    pub organization: Option<String>,
    /// Organizational Unit (OU) from certificate subject
    pub organizational_unit: Option<String>,
    /// Subject Alternative Names
    pub subject_alt_names: Vec<SanEntry>,
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: String,
    /// Certificate not-after date
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Raw certificate bytes (optional, for forwarding)
    pub cert_bytes: Option<Vec<u8>>,
}

/// Subject Alternative Name entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SanEntry {
    DnsName(String),
    IpAddress(std::net::IpAddr),
    Uri(String),
    Email(String),
}

/// Extension trait for extracting identity from TLS connection
pub trait TlsIdentityExt {
    fn client_identity(&self) -> Option<&ClientIdentity>;
    fn requires_client_cert(&self) -> bool;
}

// ============ VERIFICATION ============

/// Client certificate verifier trait
#[async_trait::async_trait]
pub trait ClientCertVerifier: Send + Sync {
    /// Verify a client certificate and extract identity
    async fn verify(&self, cert: &DerCertificate) -> Result<ClientIdentity, MtlsError>;
    
    /// Check if a certificate is revoked
    async fn is_revoked(&self, cert: &DerCertificate) -> Result<bool, MtlsError>;
}

/// Default webpki-based client certificate verifier
pub struct WebPkiClientVerifier {
    /// Root certificates trusted for client authentication
    root_certs: rustls::RootCertStore,
    /// Whether to check revocation
    check_revocation: bool,
}

// ============ ERROR TYPES ============

/// mTLS error types
#[derive(Debug, thiserror::Error)]
pub enum MtlsError {
    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),
    
    #[error("Certificate error: {0}")]
    Certificate(#[from] CertificateError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Certificate parsing error: {0}")]
    X509Parse(#[from] x509_parser::error::X509Error),
    
    #[error("Client certificate required but not provided")]
    ClientCertRequired,
    
    #[error("Client certificate verification failed: {reason}")]
    ClientCertInvalid { reason: String },
    
    #[error("Certificate expired: {expiry}")]
    CertificateExpired { expiry: chrono::DateTime<chrono::Utc> },
    
    #[error("Certificate not yet valid: {not_before}")]
    CertificateNotYetValid { not_before: chrono::DateTime<chrono::Utc> },
    
    #[error("Certificate revoked")]
    CertificateRevoked,
    
    #[error("Unknown client certificate")]
    UnknownClientCert,
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Identity extraction failed: {0}")]
    IdentityExtractionFailed(String),
}

/// Certificate-specific errors
#[derive(Debug, thiserror::Error)]
pub enum CertificateError {
    #[error("Failed to load certificate: {0}")]
    LoadFailed(String),
    
    #[error("Failed to load private key: {0}")]
    KeyLoadFailed(String),
    
    #[error("Invalid certificate format: {0}")]
    InvalidFormat(String),
    
    #[error("Certificate chain incomplete: {0}")]
    IncompleteChain(String),
    
    #[error("Key mismatch: certificate and key do not match")]
    KeyMismatch,
}
```

### Error Types

```rust
// As shown above in MtlsError enum
// Using thiserror for ergonomic error handling
// All errors implement std::error::Error for compatibility

pub type Result<T> = std::result::Result<T, MtlsError>;
```

### Traits

```rust
/// Trait for types that can provide client identity
pub trait IdentityProvider {
    fn get_identity(&self) -> Option<&ClientIdentity>;
}

/// Trait for certificate storage backends
#[async_trait::async_trait]
pub trait CertStore: Send + Sync {
    /// Get certificate by alias
    async fn get_cert(&self, alias: &str) -> Result<CertEntry>;
    /// Store certificate
    async fn store_cert(&self, alias: &str, entry: CertEntry) -> Result<()>;
    /// Delete certificate
    async fn delete_cert(&self, alias: &str) -> Result<()>;
    /// List all certificate aliases
    async fn list_aliases(&self) -> Result<Vec<String>>;
}

/// Certificate store entry
pub struct CertEntry {
    pub certificate: Vec<u8>,
    pub private_key: Vec<u8>,
    pub ca_chain: Vec<Vec<u8>>,
    pub metadata: CertMetadata,
}

pub struct CertMetadata {
    pub alias: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub rotated_from: Option<String>,
}
```

## Key Rust-Specific Changes

### 1. TLS Configuration with Type Safety

**Source Pattern:** String-based config files with runtime validation

**Rust Translation:** Strongly-typed configuration structs with compile-time guarantees

```rust
// Instead of string-based cipher names:
// config.cipher_suites = ["TLS_AES_256_GCM_SHA384", "..."]

// Use typed cipher suites:
use rustls::crypto::ring::cipher_suite;

pub struct MtlsConfig {
    pub cipher_suites: Vec<&'static rustls::SupportedCipherSuite>,
    // ...
}

impl MtlsConfig {
    pub fn secure_default() -> Self {
        Self {
            cipher_suites: vec![
                cipher_suite::TLS13_AES_256_GCM_SHA384,
                cipher_suite::TLS13_AES_128_GCM_SHA256,
            ],
            // ...
        }
    }
}
```

**Rationale:** Compile-time validation of cipher suite names, IDE autocomplete, refactoring safety.

### 2. Zero-Copy Certificate Parsing

**Source Pattern:** Parse entire certificate into owned structures

**Rust Translation:** Borrowed parsing where possible, minimizing allocations

```rust
use x509_parser::prelude::*;

pub fn extract_identity_from_cert<'a>(
    cert_der: &'a [u8]
) -> Result<ClientIdentity> {
    // Parse without cloning where possible
    let cert = X509Certificate::from_der(cert_der)?;
    // Extract fields as borrowed references
    let subject = cert.0.tbs_certificate.subject;
    // ...
}
```

**Rationale:** Reduced memory allocations in hot path, better performance.

### 3. Async-First Design

**Source Pattern:** Blocking I/O for certificate operations

**Rust Translation:** All I/O operations are async with tokio

```rust
// Certificate loading is async
pub async fn load_cert_from_path(path: &Path) -> Result<Certificate> {
    let bytes = tokio::fs::read(path).await?;
    // Parse...
}

// Even certificate rotation is async
pub async fn rotate_certificates(&self) -> Result<()> {
    let new_cert = self.fetch_new_cert().await?;
    // Atomic swap with read-write lock
}
```

**Rationale:** Non-blocking operations, better resource utilization.

### 4. Middleware-Based Authentication

**Source Pattern:** Manual certificate checking in each handler

**Rust Translation:** Tower middleware extracts and validates identity

```rust
use axum::middleware::{self, Next};
use axum::extract::Extension;

/// Middleware that requires valid client certificate
pub async fn require_client_cert<B>(
    Extension(identity): Extension<Option<ClientIdentity>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let Some(identity) = identity else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    
    // Check expiration
    if identity.expires_at <= chrono::Utc::now() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    Ok(next.run(request).await)
}

// Usage in router:
let app = Router::new()
    .route("/api/data", get(handler))
    .layer(middleware::from_fn(require_client_cert));
```

**Rationale:** Separation of concerns, no duplicate validation code.

## Ownership & Borrowing Strategy

```rust
// Certificate loading pattern:
// 1. Load bytes (owned)
// 2. Parse certificate (borrowed from bytes)
// 3. Extract identity (owned, cloned from parsed cert)
// 4. Store identity in TLS connection state

pub struct MtlsAcceptor {
    // Owned configuration
    config: Arc<MtlsConfig>,
    
    // TLS config is created once, cloned for connections
    tls_config: Arc<rustls::ServerConfig>,
}

// Identity is extracted once, stored in connection state
// Handlers borrow identity from connection state
pub struct MtlsConnectionState {
    identity: Option<ClientIdentity>,
}

// Request handlers get Extension<ClientIdentity> by reference
async fn handler(
    Extension(identity): Extension<ClientIdentity>,
) -> Response {
    // Use borrowed identity
}
```

## Concurrency Model

**Approach:** Async with shared state via Arc and RwLock

**Rationale:** 
- mTLS handshakes are I/O bound (network, crypto)
- Multiple concurrent connections need access to shared config
- Certificate rotation needs thread-safe state updates

```rust
// Shared server state
pub struct MtlsServer {
    config: Arc<MtlsConfig>,
    tls_config: Arc<rustls::ServerConfig>,
    cert_store: Arc<dyn CertStore>,
    // For atomic cert rotation
    active_certs: Arc<RwLock<ActiveCertificates>>,
}

// Certificate rotation pattern
impl MtlsServer {
    pub async fn rotate_certificates(&self) -> Result<()> {
        let new_config = self.load_new_certs().await?;
        
        // Atomic swap - acquire write lock, update, release
        let mut certs = self.active_certs.write().await;
        *certs = new_config;
        // Drop releases lock
        
        Ok(())
    }
}

// Spawning connection handlers
let server = MtlsServer::new(config).await?;
let server = Arc::new(server);

// Each connection clones Arc (cheap)
for connection in listener.incoming() {
    let server = Arc::clone(&server);
    tokio::spawn(async move {
        server.handle_connection(connection).await;
    });
}
```

## Memory Considerations

- **Stack vs. Heap:** Large types (configs, certificates) on heap via `Arc`
- **Arc for shared state:** TLS config, cert store shared across connections
- **RwLock for rotation:** Multiple readers, single writer for cert updates
- **No unsafe code:** All crypto handled by audited crates (rustls, ring)
- **Certificate caching:** Parsed certificates cached to avoid reparsing

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Missing client certificate | `ClientCertRequired` error, connection rejected |
| Expired certificate | Checked during verification, `CertificateExpired` error |
| Certificate not yet valid | Checked during verification, `CertificateNotYetValid` error |
| Revoked certificate | CRL/OCSP check, `CertificateRevoked` error |
| Certificate chain incomplete | Chain validation fails, `CertificateInvalid` error |
| Key/cert mismatch | Checked during loading, `KeyMismatch` error |
| Concurrent rotation | RwLock ensures atomic updates |
| Memory exhaustion | Bounded connection limits, backpressure |
| Panic in handler | Tokio isolation, other connections unaffected |

## Code Examples

### Example: Complete mTLS Server

```rust
// crates/mtls-examples/src/bin/server.rs

use axum::{extract::Extension, routing::get, Json, Router};
use mtls_core::{ClientIdentity, MtlsConfig, ClientAuthMode};
use mtls_server::{MtlsServer, RequireClientCert};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Load mTLS configuration
    let mtls_config = MtlsConfig {
        server_cert_path: "/etc/certs/server.crt".into(),
        server_key_path: "/etc/certs/server.key".into(),
        ca_cert_path: "/etc/certs/ca.crt".into(),
        client_auth: ClientAuthMode::Required,
        tls_versions: vec![TlsVersion::Tls13],
        cipher_suites: MtlsConfig::secure_default().cipher_suites,
    };
    
    // Create mTLS server
    let server = MtlsServer::builder(mtls_config)
        .with_certificate_rotation(true)
        .with_revocation_check(false) // Enable with CRL/OCSP
        .build()
        .await?;
    
    // Build application router
    let app = Router::new()
        .route("/api/identity", get(get_identity_handler))
        .route("/api/data", get(protected_data_handler))
        .route("/health", get(health_check))
        .layer(RequireClientCert::new()) // Middleware requires cert
        .with_state(Arc::new(server));
    
    // Start server
    let addr = "0.0.0.0:8443";
    tracing::info!("Starting mTLS server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Handler that extracts client identity
async fn get_identity_handler(
    Extension(identity): Extension<ClientIdentity>,
) -> Json<ClientIdentity> {
    Json(identity)
}

/// Protected handler - only accessible with valid client cert
async fn protected_data_handler(
    Extension(identity): Extension<ClientIdentity>,
) -> Json<serde_json::Value> {
    tracing::info!("Request from: {}", identity.common_name);
    
    Json(serde_json::json!({
        "message": "Protected data",
        "client": identity.common_name,
        "organization": identity.organization,
    }))
}

/// Health check - no client cert required
async fn health_check() -> &'static str {
    "healthy"
}
```

### Example: mTLS HTTP Client

```rust
// crates/mtls-examples/src/bin/client.rs

use mtls_client::{MtlsClient, ClientCertConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    // Configure client certificate
    let cert_config = ClientCertConfig {
        cert_path: PathBuf::from("/etc/certs/client.crt"),
        key_path: PathBuf::from("/etc/certs/client.key"),
        ca_cert_path: PathBuf::from("/etc/certs/ca.crt"),
    };
    
    // Create mTLS client
    let client = MtlsClient::builder(cert_config)
        .with_timeout(std::time::Duration::from_secs(30))
        .with_retry(true)
        .build()?;
    
    // Make authenticated request
    let url = "https://api.example.com:8443/api/data";
    
    let response = client
        .get(url)
        .await?;
    
    tracing::info!("Response status: {}", response.status());
    tracing::info!("Response body: {}", response.text().await?);
    
    // Another request with custom headers
    let response = client
        .post("https://api.example.com:8443/api/submit")
        .header("X-Custom-Header", "value")
        .body(serde_json::json!({"data": "test"}).to_string())
        .send()
        .await?;
    
    Ok(())
}
```

### Example: Certificate Generation for Testing

```rust
// crates/mtls-examples/src/bin/certgen.rs

use mtls_certs::{CertGenerator, CertConfig, KeyType};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let output_dir = PathBuf::from("./test-certs");
    tokio::fs::create_dir_all(&output_dir).await?;
    
    // Generate test CA
    let ca_config = CertConfig {
        common_name: "Test CA".to_string(),
        organization: Some("Test Org".to_string()),
        validity_days: 3650, // 10 years
        key_type: KeyType::EcdsaP256,
        is_ca: true,
        ..Default::default()
    };
    
    let (ca_cert, ca_key) = CertGenerator::generate(ca_config)?;
    ca_cert.write_to_file(output_dir.join("ca.crt"))?;
    ca_key.write_to_file(output_dir.join("ca.key"))?;
    
    // Generate server certificate
    let server_config = CertConfig {
        common_name: "server.example.com".to_string(),
        organization: Some("Test Org".to_string()),
        validity_days: 365,
        key_type: KeyType::EcdsaP256,
        is_ca: false,
        subject_alt_names: vec![
            "server.example.com".to_string(),
            "localhost".to_string(),
            "127.0.0.1".to_string(),
        ],
        ..Default::default()
    };
    
    let (server_cert, server_key) = CertGenerator::generate_signed(
        server_config,
        &ca_cert,
        &ca_key,
    )?;
    server_cert.write_to_file(output_dir.join("server.crt"))?;
    server_key.write_to_file(output_dir.join("server.key"))?;
    
    // Generate client certificate
    let client_config = CertConfig {
        common_name: "client-service".to_string(),
        organization: Some("Test Org".to_string()),
        organizational_unit: Some("Backend".to_string()),
        validity_days: 90,
        key_type: KeyType::EcdsaP256,
        is_ca: false,
        subject_alt_names: vec![
            "client-service.internal".to_string(),
        ],
        ..Default::default()
    };
    
    let (client_cert, client_key) = CertGenerator::generate_signed(
        client_config,
        &ca_cert,
        &ca_key,
    )?;
    client_cert.write_to_file(output_dir.join("client.crt"))?;
    client_key.write_to_file(output_dir.join("client.key"))?;
    
    println!("Certificates generated in {:?}", output_dir);
    println!("\nCA Certificate:    ca.crt");
    println!("Server Certificate: server.crt");
    println!("Server Key:         server.key");
    println!("Client Certificate: client.crt");
    println!("Client Key:         client.key");
    
    Ok(())
}
```

### Example: Integration Test

```rust
// tests/integration/src/main.rs

#[cfg(test)]
mod tests {
    use axum::{routing::get, Json, Router, extract::Extension};
    use mtls_core::{MtlsConfig, ClientAuthMode, TlsVersion};
    use mtls_server::MtlsServer;
    use mtls_certs::CertGenerator;
    use tokio::net::TcpListener;
    
    #[tokio::test]
    async fn test_mtls_handshake_success() -> anyhow::Result<()> {
        // Generate test certificates
        let (ca_cert, ca_key) = CertGenerator::generate_ca("Test CA")?;
        let (server_cert, server_key) = CertGenerator::generate_server(
            "localhost",
            vec!["localhost".into(), "127.0.0.1".into()],
            &ca_cert,
            &ca_key,
        )?;
        let (client_cert, client_key) = CertGenerator::generate_client(
            "test-client",
            &ca_cert,
            &ca_key,
        )?;
        
        // Write certs to temp directory
        let temp_dir = tempfile::tempdir()?;
        // ... write certs to temp_dir ...
        
        // Configure mTLS server
        let mtls_config = MtlsConfig {
            server_cert_path: temp_dir.path().join("server.crt"),
            server_key_path: temp_dir.path().join("server.key"),
            ca_cert_path: temp_dir.path().join("ca.crt"),
            client_auth: ClientAuthMode::Required,
            tls_versions: vec![TlsVersion::Tls13],
            ..Default::default()
        };
        
        let server = MtlsServer::builder(mtls_config).build().await?;
        
        let app = Router::new()
            .route("/test", get(|| async { "ok" }));
        
        // Bind to random port
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        
        // Spawn server
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await
        });
        
        // Create mTLS client
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .add_root_certificate(
                reqwest::tls::Certificate::from_pem(
                    &std::fs::read(temp_dir.path().join("ca.crt"))?
                )?
            )
            .identity(
                reqwest::Identity::from_pem(
                    &std::fs::read(temp_dir.path().join("client.crt"))?
                )?
                .key(&std::fs::read(temp_dir.path().join("client.key"))?)
            )
            .build()?;
        
        // Make request
        let response = client
            .get(format!("https://{}/test", addr))
            .send()
            .await?;
        
        assert!(response.status().is_success());
        
        // Cleanup
        server_handle.abort();
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_mtls_handshake_no_client_cert() -> anyhow::Result<()> {
        // Similar setup, but client doesn't present certificate
        // Should fail with handshake error
        // ...
        Ok(())
    }
    
    #[tokio::test]
    async fn test_mtls_expired_certificate() -> anyhow::Result<()> {
        // Generate expired certificate
        // Should fail verification
        // ...
        Ok(())
    }
}
```

## Migration Path

### Phase 1: Development Environment

1. Set up test CA with `rcgen`
2. Generate development certificates
3. Run mTLS server locally
4. Test with mTLS client

### Phase 2: Staging Deployment

1. Provision production CA (or use managed service)
2. Deploy certificate management infrastructure
3. Deploy mTLS server to staging
4. Issue client certificates to test services
5. Validate end-to-end authentication

### Phase 3: Production Rollout

1. Deploy with `ClientAuthMode::Optional` initially
2. Monitor and log all client certificate presentations
3. Verify authorization logic with real traffic
4. Switch to `ClientAuthMode::Required`
5. Enable monitoring and alerting

### Phase 4: Operational Maturity

1. Implement automated certificate rotation
2. Set up revocation checking (CRL/OCSP)
3. Configure dashboards and alerts
4. Document runbooks for cert incidents

## Performance Considerations

- **Handshake overhead:** mTLS adds ~2-3ms per handshake (TLS 1.3)
- **Session resumption:** Enable TLS session tickets to reduce handshake cost
- **Certificate chain:** Keep chains short (direct CA signing preferred)
- **OCSP stapling:** Reduces revocation check latency
- **Connection pooling:** Reuse connections to amortize handshake cost

## Testing Strategy

### Unit Tests
- Certificate parsing and identity extraction
- Authorization logic
- Configuration validation
- Error handling

### Integration Tests
- Full mTLS handshake (valid certs)
- Handshake rejection (invalid/expired/revoked certs)
- Certificate rotation during active connections
- High concurrency scenarios

### Load Tests
- Handshake throughput (handshakes/second)
- Connection saturation
- Memory usage under load
- Certificate rotation under load

## Open Considerations

1. **OCSP Stapling:** Requires additional infrastructure for OCSP responder
2. **SPIFFE/SPIRE Integration:** Consider for large-scale deployments
3. **Certificate Transparency:** Logging certs to CT logs for audit
4. **Hardware Backed Keys:** HSM integration for key storage
5. **Multi-CA Support:** Supporting multiple trust anchors

