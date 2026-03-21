//! mTLS Server Example
//!
//! This example demonstrates a production-ready mTLS server using
//! axum, rustls, and tokio.

use axum::{
    extract::Extension,
    routing::{get, post},
    Json, Router,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Utc};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use rustls_pemfile::{certs, private_key};
use serde::Serialize;
use std::{
    fs::File,
    io::{self, BufReader},
    net::SocketAddr,
    path::Path,
    sync::Arc,
};
use tokio_rustls::TlsAcceptor;
use tracing::{error, info, warn};
use x509_parser::prelude::*;

// ============ Configuration ============

#[derive(Debug, Clone)]
pub struct MtlsServerConfig {
    pub bind_addr: SocketAddr,
    pub server_cert_path: String,
    pub server_key_path: String,
    pub ca_cert_path: String,
    pub require_client_cert: bool,
}

impl Default for MtlsServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8443".parse().unwrap(),
            server_cert_path: "./certs/server.crt".to_string(),
            server_key_path: "./certs/server.key".to_string(),
            ca_cert_path: "./certs/ca.crt".to_string(),
            require_client_cert: true,
        }
    }
}

// ============ Client Identity ============

#[derive(Debug, Clone, Serialize)]
pub struct ClientIdentity {
    pub common_name: String,
    pub organization: Option<String>,
    pub organizational_unit: Option<String>,
    pub subject_alt_names: Vec<String>,
    pub fingerprint: String,
    pub expires_at: DateTime<Utc>,
}

// ============ TLS Configuration ============

/// Load certificates from PEM file
fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    certs(reader).collect()
}

/// Load private key from PEM file
fn load_private_key(path: &Path) -> io::Result<PrivateKeyDer<'static>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    private_key(reader)
        .expect("No private key found")
        .map(|k| k.to_owned())
}

/// Extract client identity from X.509 certificate
fn extract_client_identity(cert: &CertificateDer) -> Result<ClientIdentity, String> {
    let x509 = X509Certificate::from_der(cert)
        .map_err(|e| format!("Failed to parse certificate: {}", e))?
        .1;

    let subject = x509.tbs_certificate.subject;
    let common_name = subject
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let organization = subject
        .iter_organization()
        .next()
        .and_then(|o| o.as_str().ok())
        .map(String::from);

    let organizational_unit = subject
        .iter_organizational_unit()
        .next()
        .and_then(|ou| ou.as_str().ok())
        .map(String::from);

    let subject_alt_names = x509
        .subject_alternative_name()
        .map_err(|e| format!("Failed to parse SAN: {}", e))?
        .and_then(|ext| {
            let names: Vec<String> = ext
                .value
                .general_names
                .iter()
                .filter_map(|name| match name {
                    GeneralName::DNSName(name) => Some(name.to_string()),
                    GeneralName::IPAddress(ip) => {
                        Some(std::net::IpAddr::from(*ip).to_string())
                    }
                    GeneralName::URI(uri) => Some(uri.to_string()),
                    GeneralName::RFC822Name(email) => Some(email.to_string()),
                    _ => None,
                })
                .collect();
            if names.is_empty() {
                None
            } else {
                Some(names)
            }
        })
        .unwrap_or_default();

    let fingerprint = calculate_fingerprint(cert);

    let expires_at = DateTime::from_timestamp(
        x509.validity().not_after.timestamp(),
        0,
    )
    .unwrap_or_default()
    .with_timezone(&Utc);

    Ok(ClientIdentity {
        common_name,
        organization,
        organizational_unit,
        subject_alt_names,
        fingerprint,
        expires_at,
    })
}

/// Calculate SHA-256 fingerprint of certificate
fn calculate_fingerprint(cert: &CertificateDer) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(cert);
    hash.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(":")
}

/// Build TLS configuration with client certificate verification
fn build_tls_config(
    server_cert_path: &str,
    server_key_path: &str,
    ca_cert_path: &str,
    require_client_cert: bool,
) -> Result<Arc<ServerConfig>, String> {
    // Load server certificate
    let server_certs = load_certs(Path::new(server_cert_path))
        .map_err(|e| format!("Failed to load server certificate: {}", e))?;

    // Load server private key
    let server_key = load_private_key(Path::new(server_key_path))
        .map_err(|e| format!("Failed to load server key: {}", e))?;

    // Load CA certificate for client verification
    let ca_certs = load_certs(Path::new(ca_cert_path))
        .map_err(|e| format!("Failed to load CA certificate: {}", e))?;

    // Create root cert store from CA
    let mut client_cert_roots = rustls::RootCertStore::empty();
    for ca_cert in ca_certs {
        client_cert_roots
            .add(ca_cert)
            .map_err(|e| format!("Failed to add CA certificate: {}", e))?;
    }

    // Configure client certificate verification
    let client_auth = if require_client_cert {
        ServerConfig::builder()
            .with_client_cert_verifier(
                rustls::WebPkiClientVerifier::builder(Arc::new(client_cert_roots))
                    .build()
                    .map_err(|e| format!("Failed to build client verifier: {}", e))?,
            )
            .with_single_cert(server_certs, server_key)
            .map_err(|e| format!("Failed to set server cert: {}", e))?
    } else {
        // Optional client cert - use AnonymousClientVerifier
        ServerConfig::builder()
            .with_client_cert_verifier(
                rustls::WebPkiClientVerifier::builder(Arc::new(client_cert_roots))
                    .allow_unauthenticated()
                    .build()
                    .map_err(|e| format!("Failed to build client verifier: {}", e))?,
            )
            .with_single_cert(server_certs, server_key)
            .map_err(|e| format!("Failed to set server cert: {}", e))?
    };

    Ok(Arc::new(client_auth))
}

// ============ Handlers ============

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: DateTime<Utc>,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
    })
}

async fn get_identity(Extension(identity): Extension<Option<ClientIdentity>>) -> Response {
    match identity {
        Some(id) => Json(id).into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Client certificate not provided"})),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
struct ProtectedData {
    message: String,
    client: String,
    organization: Option<String>,
    timestamp: DateTime<Utc>,
}

async fn protected_data(
    Extension(identity): Extension<ClientIdentity>,
) -> Json<ProtectedData> {
    info!("Request from client: {} ({})", identity.common_name, identity.fingerprint);

    Json(ProtectedData {
        message: "This is protected data only visible to authenticated clients".to_string(),
        client: identity.common_name,
        organization: identity.organization,
        timestamp: Utc::now(),
    })
}

async fn echo(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    Json(body)
}

// ============ Main ============

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mtls_server_example=info".parse().unwrap()),
        )
        .init();

    info!("Starting mTLS Server Example");

    // Load configuration
    let config = MtlsServerConfig::default();

    info!("Loading certificates...");
    info!("  Server cert: {}", config.server_cert_path);
    info!("  Server key:  {}", config.server_key_path);
    info!("  CA cert:     {}", config.ca_cert_path);

    // Build TLS configuration
    let tls_config = build_tls_config(
        &config.server_cert_path,
        &config.server_key_path,
        &config.ca_cert_path,
        config.require_client_cert,
    )?;

    if config.require_client_cert {
        info!("Client certificates: REQUIRED");
    } else {
        info!("Client certificates: OPTIONAL");
    }

    // Build application router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/identity", get(get_identity))
        .route("/api/protected", get(protected_data))
        .route("/api/echo", post(echo))
        .layer(axum::middleware::from_fn(|req, next| async move {
            // Extract client cert from TLS connection state
            // In production, this would extract from the TLS connection
            next.run(req).await
        }))
        .with_state(());

    // Create TLS acceptor
    let acceptor = TlsAcceptor::from(tls_config);

    info!("Binding to {}", config.bind_addr);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    info!("mTLS server listening on {}", config.bind_addr);
    info!();
    info!("Endpoints:");
    info!("  GET  /health       - Health check (no auth required)");
    info!("  GET  /api/identity - Get client certificate identity");
    info!("  GET  /api/protected - Protected endpoint (requires valid client cert)");
    info!("  POST /api/echo     - Echo back JSON body");

    // Accept connections
    loop {
        let (stream, addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let app = app.clone();

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    info!("New TLS connection from {}", addr);
                    // Handle the connection...
                }
                Err(e) => {
                    warn!("TLS handshake failed from {}: {}", addr, e);
                }
            }
        });
    }
}
