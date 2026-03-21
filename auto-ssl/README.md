# Auto-SSL: Production Automatic Certificate Management

A production-grade automatic SSL/TLS certificate acquisition, renewal, and storage service implemented in Rust.

## Features

- **Multi-Provider ACME Support**: Let's Encrypt, ZeroSSL, and custom ACME servers
- **HTTP-01 and DNS-01 Challenges**: Flexible domain validation methods
- **Automatic Renewal**: Configurable renewal thresholds with exponential backoff
- **Multiple Storage Backends**: Local filesystem, AWS S3, Cloudflare R2, HashiCorp Vault
- **Zero-Downtime Rotation**: Hot-reload certificates without service interruption
- **DNS Provider Integration**: Cloudflare, Route53, GCP Cloud DNS, DigitalOcean
- **Comprehensive Monitoring**: Prometheus metrics, alerts, audit logging

## Contents

```
auto-ssl/
├── README.md                 # This file
├── exploration.md            # ACME protocol, architecture, patterns
├── rust-revision.md          # Complete Rust implementation guide
├── examples/
│   ├── embedded-server/      # Embedded Auto-SSL HTTPS server
│   ├── standalone-manager/   # CLI certificate manager
│   └── cert-manager/         # Certificate manager with S3/R2
└── tasks.md                  # Implementation tasks
```

## Quick Start

### Example 1: Embedded Server

```bash
cd examples/embedded-server
cargo run --release
```

This starts an HTTPS server that automatically acquires and manages certificates.

### Example 2: Standalone CLI Manager

```bash
cd examples/standalone-manager

# Issue a certificate
cargo run --release -- issue example.com

# Check status
cargo run --release -- status example.com

# List all certificates
cargo run --release -- list

# Run renewal daemon
cargo run --release -- daemon --interval-hours 24
```

### Example 3: Certificate Manager with S3/R2

```bash
cd examples/cert-manager

# Configure environment
export AWS_ACCESS_KEY_ID=xxx
export AWS_SECRET_ACCESS_KEY=xxx
export AWS_DEFAULT_REGION=us-east-1

# For R2:
export R2_ACCOUNT_ID=xxx
export R2_ACCESS_KEY_ID=xxx
export R2_SECRET_ACCESS_KEY=xxx

cargo run --release
```

## Configuration

```rust
use auto_ssl_core::{AutoSslConfig, DomainConfig, AcmeDirectory};

let config = AutoSslConfig {
    domains: vec![
        DomainConfig {
            domain: "example.com".to_string(),
            san: vec!["www.example.com".to_string()],
            preferred_challenges: vec![ChallengeType::Http01],
            dns_provider: None,
            wildcard: false,
        },
    ],
    acme_directory: AcmeDirectory::LetsEncryptProduction,
    account: AccountConfig {
        email: "admin@example.com".to_string(),
        key_path: Some("./account_key.pem".into()),
        generate_key: true,
    },
    storage: StorageConfig {
        primary: StorageBackend::S3(S3StorageConfig {
            bucket: "my-certs".into(),
            region: "us-east-1".into(),
            prefix: Some("production".into()),
            endpoint: None,
        }),
        backup: None,
        encrypt_keys: true,
        encryption_key_path: None,
    },
    renewal: RenewalConfig {
        renew_before_expiry_days: 30,
        retry_interval: Duration::from_secs(3600),
        max_retries: 5,
        jitter_percent: 20,
        auto_renew: true,
    },
    ..Default::default()
};
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Auto-SSL Manager                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ ACME Client  │  │ Renewal      │  │ Storage      │      │
│  │              │  │ Scheduler    │  │ Backends     │      │
│  │ - Let's Enc  │  │ - Monitor    │  │ - Local      │      │
│  │ - ZeroSSL    │  │ - Retry      │  │ - S3         │      │
│  │ - Custom     │  │ - Backoff    │  │ - R2         │      │
│  └──────────────┘  └──────────────┘  │ - Vault      │      │
│                                      └──────────────┘      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐                         │
│  │ HTTP-01      │  │ DNS-01       │                         │
│  │ Challenge    │  │ Challenges   │                         │
│  │ Server       │  │ - Cloudflare │                         │
│  │              │  │ - Route53    │                         │
│  │              │  │ - GCP DNS    │                         │
│  └──────────────┘  └──────────────┘                         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Certificate Lifecycle

```
1. REQUEST
   └── Check if certificate exists and is valid
   └── If not, initiate ACME order

2. VALIDATE
   └── HTTP-01: Serve token at /.well-known/acme-challenge/
   └── DNS-01: Create TXT record via DNS provider API
   └── Wait for CA validation

3. ISSUE
   └── Generate CSR
   └── Finalize order
   └── Download certificate

4. STORE
   └── Save to primary storage (local, S3, R2)
   └── Optionally backup to secondary storage
   └── Encrypt private keys at rest

5. MONITOR
   └── Track expiration dates
   └── Alert on approaching expiry
   └── Schedule automatic renewal

6. RENEW
   └── Trigger at configured threshold (e.g., 30 days)
   └── Zero-downtime certificate swap
   └── Notify dependent services
```

## Production Checklist

- [ ] Use production ACME endpoint (not staging)
- [ ] Configure proper email for expiration notices
- [ ] Enable encrypted key storage
- [ ] Set up S3/R2 backup storage
- [ ] Configure DNS provider for DNS-01 (if needed)
- [ ] Set up Prometheus monitoring
- [ ] Configure alert webhooks (Slack, PagerDuty)
- [ ] Test renewal process end-to-end
- [ ] Document runbooks for certificate incidents
- [ ] Set up certificate transparency monitoring

## Rate Limits (Let's Encrypt)

| Limit | Value | Mitigation |
|-------|-------|------------|
| Certificates/domain | 50/week | Cache aggressively |
| Failures | 50/hour | Exponential backoff |
| Duplicate certs | 5/week | Check existing before issuing |

## Security Considerations

- Private keys encrypted at rest (AES-256-GCM)
- Account keys stored securely and backed up
- All ACME communications over TLS 1.3
- File permissions set to 600 for private keys
- Audit logging for all certificate operations
- Support for HSM-backed key storage

## Monitoring

### Prometheus Metrics

```
auto_ssl_cert_expiry_days{domain="example.com"}     # Days until expiration
auto_ssl_acme_requests_total{status="success"}      # ACME request count
auto_ssl_acme_request_duration_seconds              # ACME operation latency
auto_ssl_storage_write_duration_seconds             # Storage operations
auto_ssl_renewal_success_total                      # Successful renewals
auto_ssl_renewal_failure_total                      # Failed renewals
```

### Alert Rules

```yaml
- alert: CertificateExpiringSoon
  expr: auto_ssl_cert_expiry_days < 14
  severity: warning

- alert: CertificateExpired
  expr: auto_ssl_cert_expiry_days < 0
  severity: critical

- alert: RenewalFailure
  expr: auto_ssl_renewal_failure_total > 0
  severity: warning
```

## Related Explorations

- [mTLS Implementation](../rust-mtls/exploration.md) - Mutual TLS with Rust
- [Security Signing](../utm-dev-production/security-signing-exploration.md) - Security patterns

## License

MIT
