//! Auto-SSL Certificate Manager with S3/R2 Storage
//!
//! This example demonstrates certificate management with
//! cloud storage backends (AWS S3, Cloudflare R2).

use chrono::{DateTime, Utc};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tracing::{error, info, warn};

// ============ Configuration ============

#[derive(Debug, Clone)]
pub struct CertManagerConfig {
    pub acme_directory: String,
    pub email: String,
    pub storage: StorageConfig,
    pub renew_days_before: u32,
}

#[derive(Debug, Clone)]
pub enum StorageConfig {
    Local { path: PathBuf },
    S3 { bucket: String, prefix: String, region: String },
    R2 { bucket: String, prefix: String, account_id: String },
}

impl Default for CertManagerConfig {
    fn default() -> Self {
        Self {
            acme_directory: acme2::LE_DIRECTORY_STAGING.to_string(),
            email: "admin@example.com".to_string(),
            storage: StorageConfig::Local { path: "./certs".into() },
            renew_days_before: 30,
        }
    }
}

// ============ Certificate Metadata ============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CertificateMetadata {
    pub domain: String,
    pub san: Vec<String>,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub acquired_at: DateTime<Utc>,
    pub provider: String,
    pub storage_path: String,
    pub checksum: String,
}

impl CertificateMetadata {
    pub fn days_remaining(&self) -> i64 {
        let now = Utc::now();
        (self.not_after - now).num_days()
    }

    pub fn needs_renewal(&self, days_before: u32) -> bool {
        self.days_remaining() <= days_before as i64
    }
}

// ============ Storage Backend ============

pub trait CertStorage: Send + Sync {
    async fn save(&self, cert: &CertificateWithKey, metadata: &CertificateMetadata) -> anyhow::Result<()>;
    async fn load(&self, domain: &str) -> anyhow::Result<Option<(CertificateWithKey, CertificateMetadata)>>;
    async fn list(&self) -> anyhow::Result<Vec<CertificateMetadata>>;
    async fn delete(&self, domain: &str) -> anyhow::Result<()>;
}

pub struct CertificateWithKey {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
    pub chain_pem: Option<Vec<u8>>,
}

// ============ Local Storage ============

pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
}

#[async_trait::async_trait]
impl CertStorage for LocalStorage {
    async fn save(&self, cert: &CertificateWithKey, metadata: &CertificateMetadata) -> anyhow::Result<()> {
        let domain_safe = metadata.domain.replace('*', "_wildcard");
        let cert_dir = self.base_path.join(&domain_safe);
        tokio::fs::create_dir_all(&cert_dir).await?;

        // Save certificate
        tokio::fs::write(cert_dir.join("cert.pem"), &cert.cert_pem).await?;
        
        // Save private key with restrictive permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::write(cert_dir.join("key.pem"), &cert.key_pem).await?;
            tokio::fs::set_permissions(cert_dir.join("key.pem"), std::fs::Permissions::from_mode(0o600)).await?;
        }
        #[cfg(not(unix))]
        {
            tokio::fs::write(cert_dir.join("key.pem"), &cert.key_pem).await?;
        }

        // Save chain if present
        if let Some(chain) = &cert.chain_pem {
            tokio::fs::write(cert_dir.join("chain.pem"), chain).await?;
        }

        // Save metadata
        let metadata_json = serde_json::to_string_pretty(metadata)?;
        tokio::fs::write(cert_dir.join("metadata.json"), metadata_json).await?;

        info!("Certificate saved to {:?}", cert_dir);
        Ok(())
    }

    async fn load(&self, domain: &str) -> anyhow::Result<Option<(CertificateWithKey, CertificateMetadata)>> {
        let domain_safe = domain.replace('*', "_wildcard");
        let cert_dir = self.base_path.join(&domain_safe);

        if !cert_dir.exists() {
            return Ok(None);
        }

        let cert_pem = tokio::fs::read(cert_dir.join("cert.pem")).await?;
        let key_pem = tokio::fs::read(cert_dir.join("key.pem")).await?;
        
        let chain_pem = tokio::fs::read(cert_dir.join("chain.pem")).await.ok();

        let metadata_json = tokio::fs::read_to_string(cert_dir.join("metadata.json")).await?;
        let metadata: CertificateMetadata = serde_json::from_str(&metadata_json)?;

        Ok(Some((CertificateWithKey { cert_pem, key_pem, chain_pem }, metadata)))
    }

    async fn list(&self) -> anyhow::Result<Vec<CertificateMetadata>> {
        let mut metadata_list = Vec::new();
        
        let mut entries = tokio::fs::read_dir(&self.base_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata_path = entry.path().join("metadata.json");
            if metadata_path.exists() {
                let content = tokio::fs::read_to_string(&metadata_path).await?;
                if let Ok(metadata) = serde_json::from_str(&content) {
                    metadata_list.push(metadata);
                }
            }
        }

        Ok(metadata_list)
    }

    async fn delete(&self, domain: &str) -> anyhow::Result<()> {
        let domain_safe = domain.replace('*', "_wildcard");
        let cert_dir = self.base_path.join(&domain_safe);
        
        if cert_dir.exists() {
            tokio::fs::remove_dir_all(&cert_dir).await?;
            info!("Certificate deleted for {}", domain);
        }

        Ok(())
    }
}

// ============ S3 Storage ============

pub struct S3Storage {
    client: aws_sdk_s3::Client,
    bucket: String,
    prefix: String,
}

impl S3Storage {
    pub async fn new(bucket: String, prefix: String, region: String) -> anyhow::Result<Self> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);
        
        Ok(Self { client, bucket, prefix })
    }

    fn key_path(&self, domain: &str, filename: &str) -> String {
        let domain_safe = domain.replace('*', "_wildcard");
        format!("{}/{}/{}", self.prefix, domain_safe, filename)
    }
}

#[async_trait::async_trait]
impl CertStorage for S3Storage {
    async fn save(&self, cert: &CertificateWithKey, metadata: &CertificateMetadata) -> anyhow::Result<()> {
        // Save certificate
        self.client.put_object()
            .bucket(&self.bucket)
            .key(self.key_path(&metadata.domain, "cert.pem"))
            .body(cert.cert_pem.clone().into())
            .send()
            .await?;

        // Save private key
        self.client.put_object()
            .bucket(&self.bucket)
            .key(self.key_path(&metadata.domain, "key.pem"))
            .body(cert.key_pem.clone().into())
            .send()
            .await?;

        // Save metadata
        let metadata_json = serde_json::to_string_pretty(metadata)?;
        self.client.put_object()
            .bucket(&self.bucket)
            .key(self.key_path(&metadata.domain, "metadata.json"))
            .body(metadata_json.into())
            .send()
            .await?;

        info!("Certificate saved to s3://{}/{}", self.bucket, self.key_path(&metadata.domain, ""));
        Ok(())
    }

    async fn load(&self, domain: &str) -> anyhow::Result<Option<(CertificateWithKey, CertificateMetadata)>> {
        // Load metadata first
        let metadata_obj = self.client.get_object()
            .bucket(&self.bucket)
            .key(self.key_path(domain, "metadata.json"))
            .send()
            .await;

        let metadata_json = match metadata_obj {
            Ok(obj) => obj.body.collect().await?.into_bytes(),
            Err(aws_sdk_s3::Error::NoSuchKey(_)) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let metadata: CertificateMetadata = serde_json::from_slice(&metadata_json)?;

        // Load certificate and key
        let cert_obj = self.client.get_object()
            .bucket(&self.bucket)
            .key(self.key_path(domain, "cert.pem"))
            .send()
            .await?;
        let cert_pem = cert_obj.body.collect().await?.into_bytes().to_vec();

        let key_obj = self.client.get_object()
            .bucket(&self.bucket)
            .key(self.key_path(domain, "key.pem"))
            .send()
            .await?;
        let key_pem = key_obj.body.collect().await?.into_bytes().to_vec();

        // Chain is optional
        let chain_pem = self.client.get_object()
            .bucket(&self.bucket)
            .key(self.key_path(domain, "chain.pem"))
            .send()
            .await
            .ok()
            .and_then(|obj| {
                obj.body.collect().await
                    .ok()
                    .map(|b| b.into_bytes().to_vec())
            });

        Ok(Some((CertificateWithKey { cert_pem, key_pem, chain_pem }, metadata)))
    }

    async fn list(&self) -> anyhow::Result<Vec<CertificateMetadata>> {
        let mut metadata_list = Vec::new();
        
        let prefix = format!("{}/", self.prefix);
        let mut paginator = self.client.list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .into_paginator()
            .send();

        while let Some(result) = paginator.next().await {
            let result = result?;
            for obj in result.contents {
                if let Some(key) = obj.key {
                    if key.ends_with("metadata.json") {
                        let get_obj = self.client.get_object()
                            .bucket(&self.bucket)
                            .key(&key)
                            .send()
                            .await?;
                        let body = get_obj.body.collect().await?.into_bytes();
                        if let Ok(metadata) = serde_json::from_slice(&body) {
                            metadata_list.push(metadata);
                        }
                    }
                }
            }
        }

        Ok(metadata_list)
    }

    async fn delete(&self, domain: &str) -> anyhow::Result<()> {
        let prefix = format!("{}/{}", self.prefix, domain.replace('*', "_wildcard"));
        
        // List all objects with this prefix
        let mut objects_to_delete = Vec::new();
        let mut paginator = self.client.list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .into_paginator()
            .send();

        while let Some(result) = paginator.next().await {
            let result = result?;
            for obj in result.contents {
                if let Some(key) = obj.key {
                    objects_to_delete.push(key);
                }
            }
        }

        // Delete objects
        if !objects_to_delete.is_empty() {
            let mut delete_builder = self.client.delete_objects()
                .bucket(&self.bucket);
            
            for key in objects_to_delete {
                delete_builder = delete_builder.delete(
                    aws_sdk_s3::types::ObjectIdentifier::builder().key(key).build()?
                );
            }
            
            delete_builder.send().await?;
            info!("Certificate deleted from S3 for {}", domain);
        }

        Ok(())
    }
}

// ============ Certificate Manager ============

pub struct CertManager {
    config: CertManagerConfig,
    storage: Arc<dyn CertStorage>,
    account: Option<acme2::Account>,
}

impl CertManager {
    pub async fn new(config: CertManagerConfig) -> anyhow::Result<Self> {
        info!("Initializing Certificate Manager...");

        // Initialize storage based on config
        let storage: Arc<dyn CertStorage> = match &config.storage {
            StorageConfig::Local { path } => {
                tokio::fs::create_dir_all(path).await?;
                Arc::new(LocalStorage::new(path.clone()))
            }
            StorageConfig::S3 { bucket, prefix, region } => {
                Arc::new(S3Storage::new(bucket.clone(), prefix.clone(), region.clone()).await?)
            }
            StorageConfig::R2 { bucket, prefix, account_id } => {
                // R2 uses S3-compatible API
                let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);
                std::env::set_var("AWS_ENDPOINT_URL", &endpoint);
                Arc::new(S3Storage::new(bucket.clone(), prefix.clone(), "auto".to_string()).await?)
            }
        };

        // Initialize ACME account
        let acme_dir = acme2::Directory::from_url(&config.acme_directory).await?;
        let account = acme_dir.create_account(&config.email, vec![], true).await?;

        Ok(Self {
            config,
            storage,
            account: Some(account),
        })
    }

    pub async fn issue_certificate(&self, domain: &str, sans: &[String]) -> anyhow::Result<CertificateMetadata> {
        info!("Issuing certificate for: {}", domain);

        let account = self.account.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ACME account not initialized"))?;

        // Check if we already have a valid certificate
        if let Some((_, existing)) = self.storage.load(domain).await? {
            if !existing.needs_renewal(self.config.renew_days_before) {
                info!("Valid certificate already exists ({} days remaining)", existing.days_remaining());
                return Ok(existing);
            }
            info!("Existing certificate needs renewal");
        }

        // Create ACME order
        let order = account.new_order(domain, sans).done().await?;
        info!("ACME order created");

        // Complete challenges
        let authzs = order.authorizations().await?;
        for authz in &authzs {
            if let Some(challenge) = authz.get_challenge() {
                challenge.validate().await?;
                info!("Challenge validated for {}", authz.identifier.value);
            }
        }

        // Wait for validation
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Generate CSR and finalize
        let mut names = vec![domain.to_string()];
        names.extend(sans.iter().cloned());

        let cert_params = rcgen::CertificateParams::new(names)?;
        let cert = rcgen::Certificate::from_params(cert_params)?;
        let csr = cert.serialize_request()?.get_der();

        let order = order.finalize(&csr).await?;

        // Get certificate
        let cert_pem = loop {
            if let Some(cert) = order.certificate().await? {
                break cert.pem();
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        };

        let key_pem = cert.serialize_private_key_pem().into_bytes();

        // Parse certificate for metadata
        let (_, x509) = x509_parser::parse_x509_certificate(cert_pem.as_bytes())?;
        let validity = x509.validity();

        let metadata = CertificateMetadata {
            domain: domain.to_string(),
            san: sans.iter().cloned().collect(),
            not_before: DateTime::from_timestamp(validity.not_before.timestamp(), 0).unwrap(),
            not_after: DateTime::from_timestamp(validity.not_after.timestamp(), 0).unwrap(),
            acquired_at: Utc::now(),
            provider: self.config.acme_directory.clone(),
            storage_path: domain.to_string(),
            checksum: format!("{:x}", md5::compute(&cert_pem)),
        };

        // Save to storage
        self.storage.save(
            &CertificateWithKey {
                cert_pem: cert_pem.into_bytes(),
                key_pem,
                chain_pem: None,
            },
            &metadata,
        ).await?;

        info!("Certificate issued successfully (expires: {})", metadata.not_after.format("%Y-%m-%d"));
        Ok(metadata)
    }

    pub async fn list_certificates(&self) -> anyhow::Result<Vec<CertificateMetadata>> {
        let certs = self.storage.list().await?;
        
        println!("\nManaged Certificates:");
        println!("{:-<70}", "");
        
        for cert in &certs {
            let status = if cert.needs_renewal(self.config.renew_days_before) {
                format!("RENEWAL NEEDED ({} days)", cert.days_remaining())
            } else {
                format!("Valid ({} days)", cert.days_remaining())
            };
            
            println!("  {:<25} Expires: {:<12} {}", cert.domain, cert.not_after.format("%Y-%m-%d"), status);
        }
        
        println!();
        Ok(certs)
    }

    pub async fn renew_certificates(&self) -> anyhow::Result<usize> {
        let certs = self.storage.list().await?;
        let mut renewed = 0;

        for cert in certs {
            if cert.needs_renewal(self.config.renew_days_before) {
                info!("Renewing certificate for {}", cert.domain);
                match self.issue_certificate(&cert.domain, &cert.san).await {
                    Ok(_) => {
                        renewed += 1;
                        info!("Renewed successfully");
                    }
                    Err(e) => {
                        error!("Failed to renew {}: {}", cert.domain, e);
                    }
                }
            }
        }

        Ok(renewed)
    }
}

// ============ Main ============

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("cert_manager=info".parse().unwrap()),
        )
        .init();

    info!("Auto-SSL Certificate Manager");
    info!("=============================");

    // Example: Local storage
    let config = CertManagerConfig {
        storage: StorageConfig::Local { path: "./managed-certs".into() },
        ..Default::default()
    };

    let manager = CertManager::new(config).await?;

    // List existing certificates
    manager.list_certificates().await?;

    // Example: Issue a certificate (staging only - won't work for real domains without validation)
    // manager.issue_certificate("example.com", &[]).await?;

    info!("Commands:");
    info!("  To issue:   manager.issue_certificate(\"domain.com\", &[]).await");
    info!("  To renew:   manager.renew_certificates().await");
    info!("  To list:    manager.list_certificates().await");

    Ok(())
}

// Required for async_trait
mod async_trait_impl {
    pub use async_trait::async_trait;
}
