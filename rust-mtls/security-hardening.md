# Security Hardening for mTLS

## Overview

This document covers security best practices, compliance considerations, audit logging, and hardening techniques for production mTLS deployments.

## Security Architecture

### Defense in Depth

```
┌─────────────────────────────────────────────────────────────┐
│                    Security Layers                           │
├─────────────────────────────────────────────────────────────┤
│ Layer          │ Controls                                   │
├────────────────┼────────────────────────────────────────────┤
│ Network        │ Network policies, service mesh, firewall   │
│ Transport      │ mTLS, cipher suite restrictions            │
│ Application    │ Certificate-based authorization, RBAC      │
│ Host           │ File permissions, secret management        │
│ Operational    │ Monitoring, audit logging, alerting        │
└─────────────────────────────────────────────────────────────┘
```

### Zero Trust Principles

```rust
// Zero trust mTLS implementation pattern
pub struct ZeroTrustConfig {
    // Every request must be authenticated
    pub require_client_cert: bool,

    // Certificate must be valid and not just present
    pub validate_cert_chain: bool,

    // Check revocation status
    pub check_revocation: bool,

    // Verify SPIFFE ID against policy
    pub authorize_identity: bool,

    // Least privilege access
    pub enforce_least_privilege: bool,
}

impl ZeroTrustConfig {
    pub fn production() -> Self {
        Self {
            require_client_cert: true,
            validate_cert_chain: true,
            check_revocation: true,
            authorize_identity: true,
            enforce_least_privilege: true,
        }
    }
}
```

## Cipher Suite Hardening

### Recommended Cipher Configuration

```rust
use rustls::SupportedCipherSuite;

/// Production-hardened cipher suite configuration
pub fn get_secure_cipher_suites() -> Vec<&'static SupportedCipherSuite> {
    vec![
        // TLS 1.3 suites (preferred)
        &rustls::cipher_suite::TLS13_AES_256_GCM_SHA384,
        &rustls::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
        &rustls::cipher_suite::TLS13_AES_128_GCM_SHA256,
    ]
}

/// Cipher suites to explicitly disable
pub const DISABLED_CIPHERS: &[&str] = &[
    "TLS_RSA_WITH_AES_128_CBC_SHA",    // No PFS
    "TLS_RSA_WITH_AES_256_CBC_SHA",    // No PFS
    "TLS_RSA_WITH_3DES_EDE_CBC_SHA",   // Weak (Sweet32)
    "TLS_RSA_WITH_RC4_128_SHA",        // Broken (RC4)
    "TLS_DHE_WITH_AES_128_CBC_SHA",    // Slow, no advantage over ECDHE
];
```

### TLS Version Policy

```rust
use rustls::ProtocolVersion;

pub struct TlsVersionPolicy {
    pub min_version: ProtocolVersion,
    pub max_version: ProtocolVersion,
}

impl TlsVersionPolicy {
    /// Production policy: TLS 1.2 minimum, TLS 1.3 preferred
    pub fn production() -> Self {
        Self {
            min_version: ProtocolVersion::TLSv12,
            max_version: ProtocolVersion::TLSv13,
        }
    }

    /// High-security policy: TLS 1.3 only
    pub fn high_security() -> Self {
        Self {
            min_version: ProtocolVersion::TLSv13,
            max_version: ProtocolVersion::TLSv13,
        }
    }
}
```

## Certificate Validation Hardening

### Strict Certificate Validation

```rust
use rustls::client::{ServerCertVerifier, ServerCertVerified};
use rustls::{Certificate, ServerName};
use std::time::SystemTime;

pub struct StrictCertVerifier {
    root_store: rustls::RootCertStore,
    require_san: bool,
    check_revocation: bool,
}

impl StrictCertVerifier {
    pub fn new(root_store: rustls::RootCertStore) -> Self {
        Self {
            root_store,
            require_san: true,
            check_revocation: false, // Enable with OCSP/CRL support
        }
    }

    pub fn with_revocation_check(mut self, enabled: bool) -> Self {
        self.check_revocation = enabled;
        self
    }
}

impl ServerCertVerifier for StrictCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &Certificate,
        intermediates: &[Certificate],
        server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        now: SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Parse and validate certificate
        use x509_parser::prelude::*;

        let cert = parse_x509_certificate(&end_entity.0)
            .map_err(|_| rustls::Error::InvalidCertificateData("Failed to parse certificate"))?
            .1;

        // Check validity period
        let current_time = time::OffsetDateTime::from_unix_timestamp(
            now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64
        ).unwrap();

        if current_time < cert.validity().not_before || current_time > cert.validity().not_after {
            return Err(rustls::Error::InvalidCertificateData("Certificate expired or not yet valid"));
        }

        // Verify SAN matches server name
        if self.require_san {
            let san_matches = cert.subject_alternative_name()
                .map(|san| {
                    san.general_names.iter().any(|name| {
                        match (server_name, name) {
                            (ServerName::DnsName(expected), GeneralName::DNSName(actual)) => {
                                actual == expected
                            }
                            (ServerName::IpAddress(expected), GeneralName::IPAddress(actual)) => {
                                actual == expected.as_ref()
                            }
                            _ => false,
                        }
                    })
                })
                .unwrap_or(false);

            if !san_matches {
                return Err(rustls::Error::InvalidCertificateData("SAN verification failed"));
            }
        }

        // Verify key usage for server auth
        if let Some(key_usage) = cert.key_usage() {
            if !key_usage.contains(x509_parser::extensions::KeyUsage::digitalSignature()) {
                return Err(rustls::Error::InvalidCertificateData("Invalid key usage"));
            }
        }

        // Verify extended key usage
        if let Some(ext_key_usage) = cert.extended_key_usage() {
            let server_auth_oid = oid_registry().oid_for_name("serverAuth").unwrap();
            if !ext_key_usage.iter().any(|oid| oid == &server_auth_oid) {
                return Err(rustls::Error::InvalidCertificateData("Missing serverAuth EKU"));
            }
        }

        Ok(ServerCertVerified::assertion())
    }
}
```

### SPIFFE ID Validation

```rust
use x509_parser::prelude::*;

pub struct SpiffeValidator {
    allowed_trust_domains: Vec<String>,
    allowed_namespaces: Vec<String>,
    allowed_service_accounts: Vec<String>,
}

impl SpiffeValidator {
    pub fn new(allowed_trust_domains: Vec<String>) -> Self {
        Self {
            allowed_trust_domains,
            allowed_namespaces: vec![],
            allowed_service_accounts: vec![],
        }
    }

    pub fn with_namespace_restriction(mut self, namespaces: Vec<String>) -> Self {
        self.allowed_namespaces = namespaces;
        self
    }

    pub fn with_service_account_restriction(mut self, sas: Vec<String>) -> Self {
        self.allowed_service_accounts = sas;
        self
    }

    pub fn validate(&self, cert: &x509_parser::certificate::Certificate) -> Result<bool, SpiffeError> {
        // Extract SPIFFE URI from SAN
        let spiffe_uri = cert.subject_alternative_name()
            .ok_or(SpiffeError::NoSAN)?
            .general_names
            .iter()
            .find_map(|name| {
                if let GeneralName::URI(uri) = name {
                    if uri.starts_with("spiffe://") {
                        Some(uri.as_str())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .ok_or(SpiffeError::NoSpiffeId)?;

        // Parse SPIFFE ID: spiffe://trust-domain/ns/namespace/sa/service-account
        let parts: Vec<&str> = spiffe_uri.trim_start_matches("spiffe://").split('/').collect();

        if parts.len() < 5 || parts[1] != "ns" || parts[3] != "sa" {
            return Err(SpiffeError::InvalidFormat(spiffe_uri.to_string()));
        }

        let trust_domain = parts[0];
        let namespace = parts[2];
        let service_account = parts[4];

        // Validate trust domain
        if !self.allowed_trust_domains.contains(&trust_domain.to_string()) {
            return Err(SpiffeError::UntrustedDomain(trust_domain.to_string()));
        }

        // Validate namespace if restricted
        if !self.allowed_namespaces.is_empty() && !self.allowed_namespaces.contains(&namespace.to_string()) {
            return Err(SpiffeError::NamespaceNotAllowed(namespace.to_string()));
        }

        // Validate service account if restricted
        if !self.allowed_service_accounts.is_empty() && !self.allowed_service_accounts.contains(&service_account.to_string()) {
            return Err(SpiffeError::ServiceAccountNotAllowed(service_account.to_string()));
        }

        Ok(true)
    }
}

#[derive(Debug)]
pub enum SpiffeError {
    NoSAN,
    NoSpiffeId,
    InvalidFormat(String),
    UntrustedDomain(String),
    NamespaceNotAllowed(String),
    ServiceAccountNotAllowed(String),
}
```

## Private Key Protection

### Secure Key Storage

```rust
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

/// Write private key with secure permissions
pub fn write_key_securely(path: &str, key_data: &[u8]) -> std::io::Result<()> {
    // Create file with 0600 permissions (owner read/write only)
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;

    file.write_all(key_data)?;
    file.sync_all()?;

    Ok(())
}

/// Verify key file permissions
pub fn verify_key_permissions(path: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path)
        .map_err(|e| format!("Failed to stat key file: {}", e))?;

    let mode = metadata.permissions().mode();

    // Check that permissions are 0600 or stricter
    if mode & 0o077 != 0 {
        return Err(format!(
            "Private key has insecure permissions: {:o} (should be 600 or stricter)",
            mode & 0o777
        ));
    }

    Ok(())
}
```

### HSM Integration (AWS KMS Example)

```rust
use aws_sdk_kms::{Client as KmsClient, types::EncryptionAlgorithmSpec};

pub struct KmsKeyManager {
    kms_client: KmsClient,
    key_id: String,
}

impl KmsKeyManager {
    pub async fn new(key_id: String) -> Result<Self, Box<dyn std::error::Error>> {
        let config = aws_config::load_from_env().await;
        let kms_client = KmsClient::new(&config);

        Ok(Self { kms_client, key_id })
    }

    /// Decrypt key material stored in KMS
    pub async fn decrypt_key(&self, encrypted_key: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let response = self.kms_client
            .decrypt()
            .ciphertext_blob(encrypted_key)
            .encryption_algorithm(EncryptionAlgorithmSpec::RsaesOaepSha256)
            .send()
            .await?;

        Ok(response.plaintext.unwrap().into_inner())
    }

    /// Generate data key for envelope encryption
    pub async fn generate_data_key(&self) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
        let response = self.kms_client
            .generate_data_key()
            .key_id(&self.key_id)
            .key_spec(aws_sdk_kms::types::DataKeySpec::Aes256)
            .send()
            .await?;

        let plaintext = response.plaintext.unwrap().into_inner();
        let encrypted = response.ciphertext_blob.unwrap().into_inner();

        Ok((plaintext, encrypted))
    }
}
```

## Audit Logging

### Security Event Logging

```rust
use tracing::{info, warn, error, Span};
use chrono::Utc;

pub struct SecurityAuditLogger {
    service_name: String,
}

impl SecurityAuditLogger {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    pub fn log_certificate_presented(
        &self,
        client_ip: &str,
        cert_subject: &str,
        cert_serial: &str,
        spiffe_id: Option<&str>,
    ) {
        info!(
            target: "security_audit",
            event = "client_certificate_presented",
            service = self.service_name,
            client_ip = %client_ip,
            cert_subject = %cert_subject,
            cert_serial = %cert_serial,
            spiffe_id = ?spiffe_id,
            timestamp = %Utc::now().to_rfc3339(),
        );
    }

    pub fn log_authentication_success(
        &self,
        client_ip: &str,
        spiffe_id: &str,
        authorized_action: &str,
    ) {
        info!(
            target: "security_audit",
            event = "authentication_success",
            service = self.service_name,
            client_ip = %client_ip,
            spiffe_id = %spiffe_id,
            authorized_action = %authorized_action,
            timestamp = %Utc::now().to_rfc3339(),
        );
    }

    pub fn log_authentication_failure(
        &self,
        client_ip: &str,
        reason: &str,
        cert_subject: Option<&str>,
    ) {
        warn!(
            target: "security_audit",
            event = "authentication_failure",
            service = self.service_name,
            client_ip = %client_ip,
            reason = %reason,
            cert_subject = ?cert_subject,
            timestamp = %Utc::now().to_rfc3339(),
        );
    }

    pub fn log_authorization_denied(
        &self,
        client_ip: &str,
        spiffe_id: &str,
        requested_resource: &str,
    ) {
        warn!(
            target: "security_audit",
            event = "authorization_denied",
            service = self.service_name,
            client_ip = %client_ip,
            spiffe_id = %spiffe_id,
            requested_resource = %requested_resource,
            timestamp = %Utc::now().to_rfc3339(),
        );
    }

    pub fn log_certificate_expired(
        &self,
        client_ip: &str,
        cert_subject: &str,
        expiry_date: &str,
    ) {
        error!(
            target: "security_audit",
            event = "certificate_expired",
            service = self.service_name,
            client_ip = %client_ip,
            cert_subject = %cert_subject,
            expiry_date = %expiry_date,
            timestamp = %Utc::now().to_rfc3339(),
        );
    }

    pub fn log_certificate_revoked(
        &self,
        client_ip: &str,
        cert_serial: &str,
        revocation_reason: &str,
    ) {
        error!(
            target: "security_audit",
            event = "certificate_revoked",
            service = self.service_name,
            client_ip = %client_ip,
            cert_serial = %cert_serial,
            revocation_reason = %revocation_reason,
            timestamp = %Utc::now().to_rfc3339(),
        );
    }
}
```

### Log Aggregation Configuration

```yaml
# Fluentd configuration for security log aggregation
apiVersion: v1
kind: ConfigMap
metadata:
  name: fluentd-config
  namespace: monitoring
data:
  fluent.conf: |
    <source>
      @type tail
      path /var/log/containers/*.log
      pos_file /var/log/fluentd-containers.log.pos
      tag kubernetes.*
      read_from_head true
      <parse>
        @type json
      </parse>
    </source>

    <filter kubernetes.**>
      @type kubernetes_metadata
      @id filter_kube_metadata
    </filter>

    # Route security audit logs
    <match kubernetes.**security_audit**>
      @type elasticsearch
      host elasticsearch.logging.svc
      port 9200
      index_name security-audit
      type_name _doc
      <buffer>
        @type file
        path /var/log/fluentd/security-audit
        flush_interval 5s
      </buffer>
    </match>
```

## Compliance Mappings

### PCI-DSS Requirements

| Requirement | mTLS Control | Implementation |
|-------------|--------------|----------------|
| 1.3.1 | Network segmentation | mTLS between cardholder data zones |
| 2.2.3 | Encryption in transit | TLS 1.2+ with strong ciphers |
| 4.1 | Strong cryptography | AES-256-GCM, ChaCha20-Poly1305 |
| 8.2.1 | Unique identities | Per-service certificates |
| 8.3.1 | Authentication | Mutual authentication required |

### HIPAA Requirements

| Requirement | mTLS Control | Implementation |
|-------------|--------------|----------------|
| 164.312(a)(1) | Access control | Certificate-based auth |
| 164.312(e)(1) | Encryption | TLS for ePHI in transit |
| 164.312(e)(2)(ii) | Integrity | TLS prevents tampering |
| 164.312(b) | Audit controls | Security event logging |

### SOC2 Controls

| Control | mTLS Contribution |
|---------|-------------------|
| CC6.1 | Logical access controls |
| CC6.6 | Encryption in transit |
| CC6.7 | Transmission integrity |
| CC7.1 | Intrusion detection |

## Security Monitoring

### Prometheus Metrics

```yaml
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: mtls-security-alerts
spec:
  groups:
    - name: mtls-security
      rules:
        # Detect mTLS handshake failures
        - alert: HighMtlsHandshakeFailures
          expr: rate(tls_handshake_total{status="failed"}[5m]) > 10
          for: 2m
          labels:
            severity: critical
          annotations:
            summary: "High rate of mTLS handshake failures"
            description: "Possible attack or misconfiguration"

        # Detect unauthorized certificate attempts
        - alert: UnauthorizedCertificateAttempts
          expr: rate(security_auth_failure_total{reason="invalid_cert"}[5m]) > 5
          for: 1m
          labels:
            severity: warning
          annotations:
            summary: "Unauthorized certificate attempts detected"

        # Detect expired certificate usage
        - alert: ExpiredCertificateUsage
          expr: rate(security_auth_failure_total{reason="cert_expired"}[5m]) > 0
          for: 1m
          labels:
            severity: critical
          annotations:
            summary: "Service attempting to use expired certificate"

        # Detect certificate from untrusted CA
        - alert: UntrustedCAAttempts
          expr: rate(security_auth_failure_total{reason="unknown_ca"}[5m]) > 3
          for: 1m
          labels:
            severity: warning
          annotations:
            summary: "Certificates from untrusted CA detected"
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "mTLS Security Monitoring",
    "panels": [
      {
        "title": "mTLS Handshake Success Rate",
        "targets": [
          {
            "expr": "sum(rate(tls_handshake_total{status=\"success\"}[5m])) / sum(rate(tls_handshake_total[5m]))"
          }
        ]
      },
      {
        "title": "Authentication Failures by Reason",
        "targets": [
          {
            "expr": "sum by (reason) (rate(security_auth_failure_total[5m]))"
          }
        ]
      },
      {
        "title": "Certificate Expiry Timeline",
        "targets": [
          {
            "expr": "tls_certificate_expiry_days"
          }
        ]
      }
    ]
  }
}
```

## Security Checklist

### Pre-Deployment

- [ ] Root CA stored offline or in HSM
- [ ] Intermediate CA used for signing
- [ ] Certificate lifetime <= 30 days for services
- [ ] Private key permissions set to 0600
- [ ] Strong cipher suites configured (TLS 1.3 preferred)
- [ ] Certificate revocation checking enabled
- [ ] SPIFFE ID validation configured
- [ ] Audit logging enabled
- [ ] Monitoring and alerting configured

### Post-Deployment

- [ ] Verify mTLS enforced on all endpoints
- [ ] Test certificate rotation
- [ ] Verify audit logs being collected
- [ ] Test alerting triggers
- [ ] Validate access controls
- [ ] Run vulnerability scan
- [ ] Document incident response procedure

### Ongoing Operations

- [ ] Review security audit logs weekly
- [ ] Monitor certificate expiry dashboard
- [ ] Test certificate revocation quarterly
- [ ] Update cipher suite config as needed
- [ ] Review and update authorization policies
- [ ] Conduct penetration testing annually

## Incident Response

### Security Incident Playbook

```markdown
## Certificate Compromise Response

### Immediate Actions (0-15 minutes)
1. Identify compromised certificate(s)
2. Revoke certificate in CA
3. Update CRL/OCSP responder
4. Alert on-call security team

### Short-term (15-60 minutes)
1. Identify affected services
2. Issue replacement certificates
3. Deploy replacement certificates
4. Verify service restoration
5. Document incident timeline

### Post-Incident (1-24 hours)
1. Conduct root cause analysis
2. Update security controls if needed
3. File incident report
4. Review and update playbooks
```
