//! Certificate Generator for mTLS Testing
//!
//! This utility generates a complete PKI hierarchy for testing mTLS:
//! - Root CA certificate
//! - Server certificate signed by CA
//! - Client certificate signed by CA

use chrono::{DateTime, Utc};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyUsagePurpose, SanType,
};
use std::{fs, path::Path};
use tracing::{error, info, warn};

// ============ Configuration ============

#[derive(Debug, Clone)]
pub struct CertGenConfig {
    pub output_dir: String,
    pub ca_validity_days: u32,
    pub server_validity_days: u32,
    pub client_validity_days: u32,
    pub ca_common_name: String,
    pub server_common_name: String,
    pub server_alt_names: Vec<String>,
    pub client_common_name: String,
    pub client_organization: String,
    pub client_organizational_unit: String,
}

impl Default for CertGenConfig {
    fn default() -> Self {
        Self {
            output_dir: "./certs".to_string(),
            ca_validity_days: 3650, // 10 years
            server_validity_days: 365, // 1 year
            client_validity_days: 90, // 90 days
            ca_common_name: "mTLS Test CA".to_string(),
            server_common_name: "localhost".to_string(),
            server_alt_names: vec![
                "localhost".to_string(),
                "127.0.0.1".to_string(),
                "::1".to_string(),
            ],
            client_common_name: "test-client".to_string(),
            client_organization: "Test Organization".to_string(),
            client_organizational_unit: "Test Unit".to_string(),
        }
    }
}

// ============ Certificate Generation ============

/// Generate a Certificate Authority (CA) certificate
fn generate_ca(params: &CertificateParams) -> Result<(Certificate, String), rcgen::Error> {
    let cert = params.self_signed(&params.key_pair)?;
    let pem = cert.pem();
    Ok((cert, pem))
}

/// Generate an end-entity certificate signed by a CA
fn generate_end_entity(
    mut params: CertificateParams,
    ca_cert: &Certificate,
    ca_key: &rcgen::KeyPair,
) -> Result<(Certificate, String), rcgen::Error> {
    params.signing_key = Some(ca_key.clone());
    let cert = params.signed_by(ca_cert)?;
    let pem = cert.pem();
    Ok((cert, pem))
}

/// Write certificate to file
fn write_cert(path: &Path, pem: &str) -> std::io::Result<()> {
    fs::write(path, pem)
}

/// Write private key to file
fn write_key(path: &Path, key: &rcgen::KeyPair) -> std::io::Result<()> {
    fs::write(path, key.serialize_pem())
}

/// Generate complete PKI hierarchy
fn generate_pki(config: &CertGenConfig) -> anyhow::Result<()> {
    info!("Creating output directory: {}", config.output_dir);
    let output_dir = Path::new(&config.output_dir);
    fs::create_dir_all(output_dir)?;

    // ==================== Generate CA ====================
    info!("Generating CA certificate...");
    
    let mut ca_dn = DistinguishedName::new();
    ca_dn.push(DnType::CommonName, &config.ca_common_name);
    ca_dn.push(DnType::OrganizationName, "Test Organization");
    
    let mut ca_params = CertificateParams::new(ca_dn)?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    ca_params.not_before = Utc::now();
    ca_params.not_after = Utc::now() + chrono::Duration::days(config.ca_validity_days as i64);
    
    let (ca_cert, ca_pem) = generate_ca(&ca_params)?;
    
    let ca_cert_path = output_dir.join("ca.crt");
    let ca_key_path = output_dir.join("ca.key");
    write_cert(&ca_cert_path, &ca_pem)?;
    write_key(&ca_key_path, &ca_cert.params().key_pair)?;
    
    info!("  CA Certificate: {}", ca_cert_path.display());
    info!("  CA Key:         {}", ca_key_path.display());
    info!("  Valid until:    {}", ca_params.not_after);

    // ==================== Generate Server Certificate ====================
    info!("Generating server certificate...");
    
    let mut server_dn = DistinguishedName::new();
    server_dn.push(DnType::CommonName, &config.server_common_name);
    server_dn.push(DnType::OrganizationName, "Test Organization");
    
    let mut server_params = CertificateParams::new(server_dn)?;
    server_params.is_ca = IsCa::NoCa;
    server_params.key_usages = vec![
        KeyUsagePurpose::KeyEncipherment,
        KeyUsagePurpose::DigitalSignature,
    ];
    server_params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ServerAuth,
    ];
    server_params.not_before = Utc::now();
    server_params.not_after = Utc::now() + chrono::Duration::days(config.server_validity_days as i64);
    
    // Add Subject Alternative Names
    for alt_name in &config.server_alt_names {
        if alt_name.parse::<std::net::IpAddr>().is_ok() {
            server_params
                .subject_alt_names
                .push(SanType::IpAddress(alt_name.clone()));
        } else {
            server_params
                .subject_alt_names
                .push(SanType::DnsName(alt_name.clone()));
        }
    }
    
    let (server_cert, server_pem) = generate_end_entity(server_params, &ca_cert, &ca_cert.params().key_pair)?;
    
    let server_cert_path = output_dir.join("server.crt");
    let server_key_path = output_dir.join("server.key");
    write_cert(&server_cert_path, &server_pem)?;
    write_key(&server_key_path, &server_cert.params().key_pair)?;
    
    info!("  Server Certificate: {}", server_cert_path.display());
    info!("  Server Key:         {}", server_key_path.display());
    info!("  SANs:               {:?}", config.server_alt_names);
    info!("  Valid until:        {}", server_params.not_after);

    // ==================== Generate Client Certificate ====================
    info!("Generating client certificate...");
    
    let mut client_dn = DistinguishedName::new();
    client_dn.push(DnType::CommonName, &config.client_common_name);
    client_dn.push(DnType::OrganizationName, &config.client_organization);
    client_dn.push(
        DnType::OrganizationalUnitName,
        &config.client_organizational_unit,
    );
    
    let mut client_params = CertificateParams::new(client_dn)?;
    client_params.is_ca = IsCa::NoCa;
    client_params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
    ];
    client_params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ClientAuth,
    ];
    client_params.not_before = Utc::now();
    client_params.not_after = Utc::now() + chrono::Duration::days(config.client_validity_days as i64);
    
    // Add client SAN
    client_params
        .subject_alt_names
        .push(SanType::DnsName(format!("{}.internal", config.client_common_name)));
    
    let (client_cert, client_pem) = generate_end_entity(client_params, &ca_cert, &ca_cert.params().key_pair)?;
    
    let client_cert_path = output_dir.join("client.crt");
    let client_key_path = output_dir.join("client.key");
    write_cert(&client_cert_path, &client_pem)?;
    write_key(&client_key_path, &client_cert.params().key_pair)?;
    
    info!("  Client Certificate: {}", client_cert_path.display());
    info!("  Client Key:         {}", client_key_path.display());
    info!("  Valid until:        {}", client_params.not_after);

    // ==================== Generate Combined CA Bundle ====================
    // For production, you might have multiple CAs in a bundle
    let ca_bundle_path = output_dir.join("ca-bundle.crt");
    write_cert(&ca_bundle_path, &ca_pem)?;
    info!("  CA Bundle:          {}", ca_bundle_path.display());

    Ok(())
}

// ============ Verification ============

/// Verify generated certificates
fn verify_certificates(config: &CertGenConfig) -> anyhow::Result<()> {
    info!("");
    info!("Verifying generated certificates...");
    
    let output_dir = Path::new(&config.output_dir);
    
    // Check all files exist
    let required_files = [
        "ca.crt",
        "ca.key",
        "server.crt",
        "server.key",
        "client.crt",
        "client.key",
        "ca-bundle.crt",
    ];
    
    for file in &required_files {
        let path = output_dir.join(file);
        if path.exists() {
            let metadata = fs::metadata(&path)?;
            info!("  ✓ {} ({} bytes)", file, metadata.len());
        } else {
            error!("  ✗ {} - MISSING", file);
            return Err(anyhow::anyhow!("Missing file: {}", file));
        }
    }
    
    // Parse and display certificate info
    info!("");
    info!("Certificate Details:");
    
    for (cert_file, description) in [
        ("ca.crt", "CA Certificate"),
        ("server.crt", "Server Certificate"),
        ("client.crt", "Client Certificate"),
    ] {
        let path = output_dir.join(cert_file);
        let pem = fs::read_to_string(&path)?;
        
        // Use rcgen to parse for basic info
        // In production, use x509-parser for more details
        info!("  {}: {}", description, cert_file);
    }
    
    Ok(())
}

// ============ Main ============

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mtls_certgen=info".parse().unwrap()),
        )
        .init();

    info!("mTLS Certificate Generator");
    info!("==========================");
    info!("");

    // Load configuration
    let config = CertGenConfig::default();
    
    info!("Configuration:");
    info!("  Output directory:    {}", config.output_dir);
    info!("  CA validity:         {} days", config.ca_validity_days);
    info!("  Server validity:     {} days", config.server_validity_days);
    info!("  Client validity:     {} days", config.client_validity_days);
    info!("  CA CN:               {}", config.ca_common_name);
    info!("  Server CN:           {}", config.server_common_name);
    info!("  Client CN:           {}", config.client_common_name);
    info!("");

    // Generate PKI
    match generate_pki(&config) {
        Ok(()) => {
            info!("");
            info!("PKI generation successful!");
        }
        Err(e) => {
            error!("PKI generation failed: {}", e);
            return Err(e);
        }
    }

    // Verify
    if let Err(e) = verify_certificates(&config) {
        error!("Certificate verification failed: {}", e);
        return Err(e);
    }

    info!("");
    info!("Next steps:");
    info!("  1. Copy ca.crt to your trusted CA store");
    info!("  2. Configure server with server.crt and server.key");
    info!("  3. Configure clients with client.crt, client.key, and ca.crt");
    info!("");
    info!("For production use:");
    info!("  - Replace with certificates from your CA");
    info!("  - Use longer key lengths (RSA 4096 or ECDSA P-384)");
    info!("  - Implement proper certificate rotation");

    Ok(())
}
