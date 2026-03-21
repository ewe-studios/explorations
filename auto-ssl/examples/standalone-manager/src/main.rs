//! Auto-SSL Standalone Certificate Manager
//!
//! A CLI tool for managing SSL certificates with automatic
//! acquisition, renewal, and storage.

use clap::{Parser, Subcommand};
use std::{path::PathBuf, time::Duration};
use tracing::{error, info, warn};

// ============ CLI Commands ============

#[derive(Parser, Debug)]
#[command(name = "auto-ssl")]
#[command(about = "Automatic SSL certificate management")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// ACME directory URL
    #[arg(long, default_value = "https://acme-staging-v02.api.letsencrypt.org/directory")]
    acme_directory: String,
    
    /// Account email
    #[arg(long)]
    email: Option<String>,
    
    /// Storage directory
    #[arg(long, default_value = "./certs")]
    storage: PathBuf,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Issue a new certificate
    Issue {
        /// Domain name
        #[arg(required = true)]
        domain: String,
        
        /// Subject Alternative Names
        #[arg(long = "san")]
        sans: Vec<String>,
        
        /// Use DNS-01 challenge
        #[arg(long)]
        dns_challenge: bool,
    },
    
    /// Renew existing certificate
    Renew {
        /// Domain name
        domain: String,
    },
    
    /// List managed certificates
    List {
        /// Show expired certificates
        #[arg(long)]
        include_expired: bool,
    },
    
    /// Check certificate status
    Status {
        /// Domain name
        domain: String,
    },
    
    /// Revoke a certificate
    Revoke {
        /// Domain name
        domain: String,
    },
    
    /// Run renewal daemon
    Daemon {
        /// Check interval (hours)
        #[arg(long, default_value = "24")]
        interval_hours: u64,
    },
}

// ============ Certificate Manager ============

struct CertificateManager {
    acme_directory: String,
    email: String,
    storage_path: PathBuf,
    account: Option<acme2::Account>,
}

impl CertificateManager {
    async fn new(acme_directory: &str, email: &str, storage_path: &PathBuf) -> anyhow::Result<Self> {
        info!("Initializing Certificate Manager...");
        
        // Create storage directory
        tokio::fs::create_dir_all(storage_path).await?;
        
        // Initialize ACME
        let acme_dir = acme2::Directory::from_url(acme_directory).await?;
        
        // Try to load existing account or create new one
        let account_path = storage_path.join("account.pem");
        let account = if account_path.exists() {
            info!("Loading existing ACME account...");
            let key_pem = tokio::fs::read_to_string(&account_path).await?;
            acme_dir.get_account(&key_pem).await?
        } else {
            info!("Creating new ACME account...");
            let account = acme_dir.create_account(email, vec![], true).await?;
            
            // Save account key
            let key_pem = account.private_key().get_pem();
            tokio::fs::write(&account_path, key_pem).await?;
            info!("Account key saved to {:?}", account_path);
            
            account
        };
        
        Ok(Self {
            acme_directory: acme_directory.to_string(),
            email: email.to_string(),
            storage_path: storage_path.clone(),
            account: Some(account),
        })
    }

    async fn issue_certificate(&self, domain: &str, sans: &[String], dns_challenge: bool) -> anyhow::Result<()> {
        info!("Issuing certificate for: {}", domain);
        
        let account = self.account.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ACME account not initialized"))?;
        
        // Create order
        let mut builder = account.new_order(domain, sans);
        let order = builder.done().await?;
        
        info!("Order created for {}", domain);
        
        // Get authorizations
        let authzs = order.authorizations().await?;
        
        for authz in &authzs {
            info!("Processing authorization: {}", authz.identifier.value);
            
            // Select challenge type
            let challenge = if dns_challenge {
                authz.get_challenge(acme2::ChallengeType::Dns01)
            } else {
                authz.get_challenge(acme2::ChallengeType::Http01)
            };
            
            let challenge = challenge.ok_or_else(|| {
                anyhow::anyhow!("No suitable challenge available")
            })?;
            
            match challenge {
                acme2::Challenge::Dns01(dns) => {
                    info!("DNS-01 challenge: {}", dns.dns_token);
                    info!("Create TXT record: _acme-challenge.{} with value: {}", 
                          authz.identifier.value, dns.dns_token);
                    info!("");
                    info!("After creating the DNS record, the challenge will validate automatically.");
                    
                    // In production, use DNS provider API to create record
                    // dns_provider.create_txt_record(...).await?;
                    
                    challenge.validate().await?;
                }
                acme2::Challenge::Http01(http) => {
                    info!("HTTP-01 challenge:");
                    info!("  URL: http://{}/.well-known/acme-challenge/{}", 
                          authz.identifier.value, http.http_token);
                    info!("  Response: {}", http.key_authorization);
                    info!("");
                    info!("In production, serve this response at the URL above.");
                    
                    challenge.validate().await?;
                }
                _ => {
                    warn!("Unsupported challenge type");
                }
            }
        }
        
        // Wait for validation
        info!("Waiting for challenge validation...");
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // Generate CSR
        let mut names = vec![domain.to_string()];
        names.extend(sans.iter().cloned());
        
        let cert_params = rcgen::CertificateParams::new(names)?;
        let cert = rcgen::Certificate::from_params(cert_params)?;
        let csr = cert.serialize_request()?.get_der();
        
        // Finalize order
        let order = order.finalize(&csr).await?;
        info!("Order finalized");
        
        // Wait for certificate
        info!("Waiting for certificate...");
        let mut retries = 0;
        let cert_pem = loop {
            if let Some(cert) = order.certificate().await? {
                break cert.pem();
            }
            retries += 1;
            if retries > 30 {
                anyhow::bail!("Certificate issuance timeout");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        };
        
        // Save certificate
        let domain_safe = domain.replace('*', "_wildcard");
        let cert_path = self.storage_path.join(format!("{}.crt", domain_safe));
        let key_path = self.storage_path.join(format!("{}.key", domain_safe));
        
        tokio::fs::write(&cert_path, &cert_pem).await?;
        tokio::fs::write(&key_path, cert.serialize_private_key_pem()).await?;
        
        info!("Certificate saved to: {:?}", cert_path);
        info!("Private key saved to: {:?}", key_path);
        
        Ok(())
    }

    async fn list_certificates(&self) -> anyhow::Result<()> {
        let mut entries = tokio::fs::read_dir(&self.storage_path).await?;
        
        println!("\nManaged Certificates:");
        println!("{:-<60}", "");
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "crt") {
                let content = tokio::fs::read_to_string(&path).await?;
                
                // Parse certificate
                match x509_parser::parse_x509_certificate(content.as_bytes()) {
                    Ok((_, cert)) => {
                        let domain = path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown");
                        
                        let validity = cert.tbs_certificate.validity;
                        let not_after = chrono::DateTime::from_timestamp(
                            validity.not_after.timestamp(), 0
                        ).unwrap_or_default();
                        
                        let days_remaining = (not_after - chrono::Utc::now()).num_days();
                        
                        let status = if days_remaining < 0 {
                            format!("EXPIRED ({} days ago)", -days_remaining)
                        } else if days_remaining < 30 {
                            format!("EXPIRING SOON ({} days)", days_remaining)
                        } else {
                            format!("Valid ({} days)", days_remaining)
                        };
                        
                        println!("  {:<30} {}", domain, status);
                    }
                    Err(e) => {
                        warn!("Failed to parse {:?}: {}", path, e);
                    }
                }
            }
        }
        
        println!();
        Ok(())
    }

    async fn check_status(&self, domain: &str) -> anyhow::Result<()> {
        let domain_safe = domain.replace('*', "_wildcard");
        let cert_path = self.storage_path.join(format!("{}.crt", domain_safe));
        
        if !cert_path.exists() {
            println!("No certificate found for: {}", domain);
            return Ok(());
        }
        
        let content = tokio::fs::read_to_string(&cert_path).await?;
        
        match x509_parser::parse_x509_certificate(content.as_bytes()) {
            Ok((_, cert)) => {
                let validity = cert.tbs_certificate.validity;
                let not_before = chrono::DateTime::from_timestamp(
                    validity.not_before.timestamp(), 0
                ).unwrap_or_default();
                let not_after = chrono::DateTime::from_timestamp(
                    validity.not_after.timestamp(), 0
                ).unwrap_or_default();
                
                let days_remaining = (not_after - chrono::Utc::now()).num_days();
                
                println!("\nCertificate Status: {}", domain);
                println!("{:-<40}", "");
                println!("  Issued:         {}", not_before.format("%Y-%m-%d %H:%M:%S"));
                println!("  Expires:        {}", not_after.format("%Y-%m-%d %H:%M:%S"));
                println!("  Days Remaining: {}", days_remaining);
                println!("  Status:         {}", 
                    if days_remaining < 0 { "EXPIRED" }
                    else if days_remaining < 30 { "RENEWAL RECOMMENDED" }
                    else { "VALID" }
                );
                
                // Subject
                let subject = &cert.tbs_certificate.subject;
                if let Some(cn) = subject.iter_common_name().next() {
                    if let Ok(cn_str) = cn.as_str() {
                        println!("  Common Name:    {}", cn_str);
                    }
                }
                
                // SANs
                if let Ok(Some(ext)) = cert.subject_alternative_name() {
                    let sans: Vec<_> = ext.value.general_names.iter()
                        .filter_map(|name| {
                            if let x509_parser::extensions::GeneralName::DNSName(name) = name {
                                Some(*name)
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !sans.is_empty() {
                        println!("  SANs:           {}", sans.join(", "));
                    }
                }
                
                println!();
            }
            Err(e) => {
                error!("Failed to parse certificate: {}", e);
            }
        }
        
        Ok(())
    }
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

    let cli = Cli::parse();

    // Get email from args or environment
    let email = cli.email
        .or_else(|| std::env::var("AUTO_SSL_EMAIL").ok())
        .unwrap_or_else(|| "admin@localhost".to_string());

    // Create manager
    let manager = CertificateManager::new(&cli.acme_directory, &email, &cli.storage).await?;

    match cli.command {
        Commands::Issue { domain, sans, dns_challenge } => {
            manager.issue_certificate(&domain, &sans, dns_challenge).await?;
        }
        
        Commands::Renew { domain } => {
            info!("Renewing certificate for: {}", domain);
            // For renewal, we'd check existing cert and issue new one
            manager.issue_certificate(&domain, &[], false).await?;
        }
        
        Commands::List { include_expired: _ } => {
            manager.list_certificates().await?;
        }
        
        Commands::Status { domain } => {
            manager.check_status(&domain).await?;
        }
        
        Commands::Revoke { domain: _ } => {
            warn!("Certificate revocation not yet implemented");
            info!("To revoke, use the ACME provider's web interface or API");
        }
        
        Commands::Daemon { interval_hours } => {
            info!("Starting renewal daemon (interval: {} hours)", interval_hours);
            info!("Press Ctrl+C to stop");
            
            let mut interval = tokio::time::interval(Duration::from_secs(interval_hours * 3600));
            
            loop {
                interval.tick().await;
                info!("Checking for certificates needing renewal...");
                // In production, scan certificates and renew those expiring soon
            }
        }
    }

    Ok(())
}
