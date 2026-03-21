//! mTLS Client Example
//!
//! This example demonstrates an HTTP client that authenticates
//! to servers using mutual TLS (client certificates).

use reqwest::{tls, Client, Response};
use rustls_pemfile::{certs, private_key};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, path::Path};
use tracing::{error, info, warn};

// ============ Configuration ============

#[derive(Debug, Clone)]
pub struct MtlsClientConfig {
    pub base_url: String,
    pub client_cert_path: String,
    pub client_key_path: String,
    pub ca_cert_path: String,
    pub timeout_secs: u64,
}

impl Default for MtlsClientConfig {
    fn default() -> Self {
        Self {
            base_url: "https://localhost:8443".to_string(),
            client_cert_path: "./certs/client.crt".to_string(),
            client_key_path: "./certs/client.key".to_string(),
            ca_cert_path: "./certs/ca.crt".to_string(),
            timeout_secs: 30,
        }
    }
}

// ============ Identity Response ============

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientIdentity {
    pub common_name: String,
    pub organization: Option<String>,
    pub organizational_unit: Option<String>,
    pub subject_alt_names: Vec<String>,
    pub fingerprint: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProtectedData {
    pub message: String,
    pub client: String,
    pub organization: Option<String>,
    pub timestamp: String,
}

// ============ mTLS Client ============

pub struct MtlsClient {
    client: Client,
    base_url: String,
}

impl MtlsClient {
    /// Build an mTLS-enabled HTTP client
    pub async fn new(config: &MtlsClientConfig) -> anyhow::Result<Self> {
        info!("Loading client certificates...");
        
        // Load client certificate
        let cert_file = File::open(&config.client_cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let client_certs: Vec<_> = certs(&mut cert_reader).collect::<Result<_, _>>()?;
        
        // Load client private key
        let key_file = File::open(&config.client_key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let client_key = private_key(&mut key_reader)?
            .expect("No private key found")
            .to_owned();
        
        // Load CA certificate for server verification
        let ca_file = File::open(&config.ca_cert_path)?;
        let mut ca_reader = BufReader::new(ca_file);
        let ca_certs: Vec<_> = certs(&mut ca_reader).collect::<Result<_, _>>()?;
        
        // Create identity from client cert and key
        let identity = tls::Identity::from_pem(
            &std::fs::read(&config.client_cert_path)?,
            &std::fs::read(&config.client_key_path)?,
        )?;
        
        // Create CA certificate bundle
        let mut ca_bundle = tls::RootCertStore::empty();
        for cert in ca_certs {
            let cert = tls::Certificate::from_der(&cert)?;
            ca_bundle.add(cert)?;
        }
        
        // Build HTTP client with mTLS
        let client = Client::builder()
            .use_rustls_tls()
            .add_root_certificate(ca_bundle)
            .identity(identity)
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()?;
        
        info!("mTLS client configured successfully");
        
        Ok(Self {
            client,
            base_url: config.base_url.clone(),
        })
    }
    
    /// Health check endpoint (may not require client cert)
    pub async fn health(&self) -> anyhow::Result<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Health check failed with status: {}",
                response.status()
            ));
        }
        
        Ok(response.json::<HealthResponse>().await?)
    }
    
    /// Get client identity as seen by the server
    pub async fn get_identity(&self) -> anyhow::Result<ClientIdentity> {
        let url = format!("{}/api/identity", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        match response.status() {
            reqwest::StatusCode::OK => Ok(response.json::<ClientIdentity>().await?),
            reqwest::StatusCode::UNAUTHORIZED => {
                Err(anyhow::anyhow!("Unauthorized - client certificate rejected"))
            }
            status => Err(anyhow::anyhow!("Request failed with status: {}", status)),
        }
    }
    
    /// Access protected data (requires valid client certificate)
    pub async fn get_protected_data(&self) -> anyhow::Result<ProtectedData> {
        let url = format!("{}/api/protected", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        match response.status() {
            reqwest::StatusCode::OK => Ok(response.json::<ProtectedData>().await?),
            reqwest::StatusCode::UNAUTHORIZED => {
                Err(anyhow::anyhow!("Unauthorized - client certificate rejected"))
            }
            status => Err(anyhow::anyhow!("Request failed with status: {}", status)),
        }
    }
    
    /// Echo data to server
    pub async fn echo<T: Serialize>(&self, data: &T) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/api/echo", self.base_url);
        let response = self.client.post(&url).json(data).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Echo request failed with status: {}",
                response.status()
            ));
        }
        
        Ok(response.json::<serde_json::Value>().await?)
    }
}

// ============ Main ============

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mtls_client_example=info".parse().unwrap()),
        )
        .init();

    info!("Starting mTLS Client Example");

    // Load configuration
    let config = MtlsClientConfig::default();
    
    info!("Configuration:");
    info!("  Base URL:       {}", config.base_url);
    info!("  Client cert:    {}", config.client_cert_path);
    info!("  Client key:     {}", config.client_key_path);
    info!("  CA cert:        {}", config.ca_cert_path);
    info!("  Timeout:        {}s", config.timeout_secs);

    // Create mTLS client
    let client = match MtlsClient::new(&config).await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create mTLS client: {}", e);
            error!("Make sure certificates exist in ./certs/");
            error!("Run the certgen example to generate test certificates.");
            return Err(e);
        }
    };

    info!("");
    info!("Testing mTLS connection to server...");
    info!("");

    // Test 1: Health check
    info!("1. Testing health endpoint...");
    match client.health().await {
        Ok(health) => {
            info!("   ✓ Health: {} (timestamp: {})", health.status, health.timestamp);
        }
        Err(e) => {
            warn!("   ✗ Health check failed: {}", e);
        }
    }

    // Test 2: Get identity
    info!("2. Fetching server's view of our identity...");
    match client.get_identity().await {
        Ok(identity) => {
            info!("   ✓ Common Name:    {}", identity.common_name);
            if let Some(org) = &identity.organization {
                info!("   ✓ Organization:   {}", org);
            }
            if let Some(ou) = &identity.organizational_unit {
                info!("   ✓ Org Unit:       {}", ou);
            }
            info!("   ✓ SANs:           {:?}", identity.subject_alt_names);
            info!("   ✓ Fingerprint:    {}", identity.fingerprint);
            info!("   ✓ Expires:        {}", identity.expires_at);
        }
        Err(e) => {
            warn!("   ✗ Identity fetch failed: {}", e);
        }
    }

    // Test 3: Access protected data
    info!("3. Accessing protected data...");
    match client.get_protected_data().await {
        Ok(data) => {
            info!("   ✓ Message: {}", data.message);
            info!("   ✓ Server sees us as: {}", data.client);
            if let Some(org) = &data.organization {
                info!("   ✓ Organization: {}", org);
            }
        }
        Err(e) => {
            warn!("   ✗ Protected data access failed: {}", e);
        }
    }

    // Test 4: Echo
    info!("4. Testing echo endpoint...");
    let test_data = serde_json::json!({
        "test": "mTLS connection",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    match client.echo(&test_data).await {
        Ok(response) => {
            info!("   ✓ Server echoed back: {}", response);
        }
        Err(e) => {
            warn!("   ✗ Echo failed: {}", e);
        }
    }

    info!("");
    info!("mTLS client test complete!");

    Ok(())
}
