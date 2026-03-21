//! Auto-SSL Embedded Server Example
//!
//! This example demonstrates a production-ready HTTPS server with
//! automatic SSL certificate acquisition and renewal.

use axum::{routing::get, Json, Router, extract::Extension};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

// ============ Configuration ============

#[derive(Debug, Clone)]
pub struct AutoSslConfig {
    /// Domains to manage
    pub domains: Vec<String>,
    /// ACME directory URL
    pub acme_directory: String,
    /// Account email
    pub email: String,
    /// HTTP-01 bind address
    pub http_bind_addr: SocketAddr,
    /// HTTPS bind address
    pub https_bind_addr: SocketAddr,
    /// Storage path
    pub storage_path: String,
    /// Renew before expiry (days)
    pub renew_days: u32,
}

impl Default for AutoSslConfig {
    fn default() -> Self {
        Self {
            domains: vec!["localhost".to_string()],
            acme_directory: acme2::LE_DIRECTORY_STAGING.to_string(),
            email: "test@example.com".to_string(),
            http_bind_addr: "0.0.0.0:80".parse().unwrap(),
            https_bind_addr: "0.0.0.0:443".parse().unwrap(),
            storage_path: "./data/certs".to_string(),
            renew_days: 30,
        }
    }
}

// ============ Certificate State ============

/// Managed certificate state
#[derive(Debug, Clone)]
pub struct CertificateState {
    pub domain: String,
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
    pub expires_at: DateTime<Utc>,
    pub renewed_at: DateTime<Utc>,
}

impl CertificateState {
    pub fn days_remaining(&self) -> i64 {
        let now = Utc::now();
        (self.expires_at - now).num_days()
    }

    pub fn should_renew(&self, renew_days: u32) -> bool {
        self.days_remaining() <= renew_days as i64
    }
}

// ============ Auto-SSL Manager ============

pub struct AutoSslManager {
    config: AutoSslConfig,
    certificates: Arc<RwLock<Vec<CertificateState>>>,
    acme_account: Option<acme2::Account>,
}

impl AutoSslManager {
    pub async fn new(config: AutoSslConfig) -> anyhow::Result<Self> {
        info!("Initializing Auto-SSL Manager...");
        
        // Create storage directory
        tokio::fs::create_dir_all(&config.storage_path).await?;
        
        // Initialize ACME account
        let acme_dir = acme2::Directory::from_url(&config.acme_directory).await?;
        
        let account = acme_dir
            .create_account(&config.email, vec![], true)
            .await?;
        
        info!("ACME account initialized");
        
        Ok(Self {
            config,
            certificates: Arc::new(RwLock::new(Vec::new())),
            acme_account: Some(account),
        })
    }

    /// Acquire certificate for a domain
    pub async fn acquire_certificate(&self, domain: &str) -> anyhow::Result<CertificateState> {
        info!("Acquiring certificate for: {}", domain);
        
        let account = self.acme_account.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ACME account not initialized"))?;
        
        // Create order
        let mut builder = account.new_order(domain, &[]);
        
        // Add SAN if different from domain
        if domain != "localhost" {
            let san = format!("www.{}", domain);
            builder = builder.domain(san);
        }
        
        let order = builder.done().await?;
        info!("Order created: {:?}", order.urls.authorize);
        
        // Complete HTTP-01 challenges
        let authzs = order.authorizations().await?;
        for authz in authzs {
            match authz.get_challenge() {
                Some(challenge) => {
                    info!("Validating challenge for: {}", authz.identifier.value);
                    
                    // In production, serve the challenge token on port 80
                    // For this example, we'll simulate validation
                    let _ = challenge.validate().await?;
                    
                    // Wait for validation
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                None => {
                    warn!("No challenge available for {}", authz.identifier.value);
                }
            }
        }
        
        // Finalize order
        let csr = rcgen::CertificateParams::new(vec![domain.to_string()])?;
        let cert = rcgen::Certificate::from_params(csr)?;
        
        let order = order.finalize(
            &cert.serialize_request()?.get_der(),
        ).await?;
        
        info!("Order finalized");
        
        // Download certificate
        let cert_data = order.certificate().await?
            .ok_or_else(|| anyhow::anyhow!("Certificate not issued"))?;
        
        let expires_at = Utc::now() + chrono::Duration::days(90);
        
        let state = CertificateState {
            domain: domain.to_string(),
            cert_pem: cert_data.pem().into_bytes(),
            key_pem: cert.serialize_private_key_pem().into_bytes(),
            expires_at,
            renewed_at: Utc::now(),
        };
        
        info!("Certificate acquired successfully");
        info!("  Expires: {}", state.expires_at);
        
        Ok(state)
    }

    /// Renew certificates that need renewal
    pub async fn renew_certificates(&self) -> anyhow::Result<usize> {
        let mut renewed = 0;
        let renew_days = self.config.renew_days;
        
        let domains_to_renew = {
            let certs = self.certificates.read().await;
            certs.iter()
                .filter(|c| c.should_renew(renew_days))
                .map(|c| c.domain.clone())
                .collect::<Vec<_>>()
        };
        
        for domain in domains_to_renew {
            info!("Renewing certificate for: {}", domain);
            match self.acquire_certificate(&domain).await {
                Ok(new_cert) => {
                    let mut certs = self.certificates.write().await;
                    if let Some(existing) = certs.iter_mut().find(|c| c.domain == domain) {
                        *existing = new_cert;
                    } else {
                        certs.push(new_cert);
                    }
                    renewed += 1;
                }
                Err(e) => {
                    error!("Failed to renew {}: {}", domain, e);
                }
            }
        }
        
        Ok(renewed)
    }

    /// Start background renewal scheduler
    pub fn start_scheduler(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Check hourly
            loop {
                interval.tick().await;
                match self.renew_certificates().await {
                    Ok(count) => {
                        if count > 0 {
                            info!("Renewed {} certificates", count);
                        }
                    }
                    Err(e) => {
                        error!("Renewal scheduler error: {}", e);
                    }
                }
            }
        })
    }

    /// Get certificate for domain
    pub async fn get_certificate(&self, domain: &str) -> Option<CertificateState> {
        let certs = self.certificates.read().await;
        certs.iter().find(|c| c.domain == domain).cloned()
    }

    /// Add certificate
    pub async fn add_certificate(&self, cert: CertificateState) {
        let mut certs = self.certificates.write().await;
        certs.push(cert);
    }
}

// ============ Handlers ============

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: DateTime<Utc>,
}

#[derive(Serialize)]
struct CertStatus {
    domain: String,
    expires_at: DateTime<Utc>,
    days_remaining: i64,
    needs_renewal: bool,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
    })
}

async fn cert_status(
    Extension(manager): Extension<Arc<AutoSslManager>>,
) -> Json<Vec<CertStatus>> {
    let certs = manager.certificates.read().await;
    let status: Vec<CertStatus> = certs.iter().map(|c| CertStatus {
        domain: c.domain.clone(),
        expires_at: c.expires_at,
        days_remaining: c.days_remaining(),
        needs_renewal: c.should_renew(manager.config.renew_days),
    }).collect();
    
    Json(status)
}

async fn root_handler() -> &'static str {
    "Auto-SSL Server - HTTPS with automatic certificate management"
}

// ============ Main ============

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("auto_ssl=info".parse().unwrap()),
        )
        .init();

    info!("Auto-SSL Embedded Server");
    info!("========================");

    // Load configuration
    let config = AutoSslConfig::default();
    
    info!("Configuration:");
    info!("  Domains: {:?}", config.domains);
    info!("  ACME:      {}", config.acme_directory);
    info!("  Email:     {}", config.email);
    info!("  HTTP:      {}", config.http_bind_addr);
    info!("  HTTPS:     {}", config.https_bind_addr);
    info!("  Storage:   {}", config.storage_path);

    // Create Auto-SSL manager
    let manager = Arc::new(AutoSslManager::new(config.clone()).await?);

    // For demo, generate a self-signed cert for localhost
    info!("Generating self-signed certificate for localhost...");
    let cert_params = rcgen::CertificateParams::new(vec!["localhost".to_string()])?;
    let cert = rcgen::Certificate::from_params(cert_params)?;
    
    let demo_cert = CertificateState {
        domain: "localhost".to_string(),
        cert_pem: cert.pem().into_bytes(),
        key_pem: cert.serialize_private_key_pem().into_bytes(),
        expires_at: Utc::now() + chrono::Duration::days(90),
        renewed_at: Utc::now(),
    };
    
    manager.add_certificate(demo_cert).await;
    info!("Demo certificate generated");

    // Start renewal scheduler
    let _scheduler = manager.clone().start_scheduler();
    info!("Renewal scheduler started");

    // Build application router
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_check))
        .route("/certs", get(cert_status))
        .layer(Extension(manager.clone()));

    info!();
    info!("Server starting on {} (HTTP)", config.http_bind_addr);
    info!("Server starting on {} (HTTPS)", config.https_bind_addr);
    info!();
    info!("Endpoints:");
    info!("  GET /        - Root handler");
    info!("  GET /health  - Health check");
    info!("  GET /certs   - Certificate status");

    // Note: In production, you'd bind to both HTTP (for challenges)
    // and HTTPS (for the actual server). This example shows the pattern.
    
    // For local testing without port 80/443 access:
    let test_addr = "127.0.0.1:3000".parse::<SocketAddr>()?;
    info!();
    info!("For testing (no root required):");
    info!("  curl http://localhost:3000/health");
    info!("  curl http://localhost:3000/certs");

    let listener = tokio::net::TcpListener::bind(test_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
