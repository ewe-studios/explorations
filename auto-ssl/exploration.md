---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/auto-ssl
repository: N/A - Greenfield exploration
explored_at: 2026-03-21
---

# Production-Grade Automatic SSL Certificate Management

## Overview

This exploration details the design and implementation of a production-grade automatic SSL certificate acquisition, renewal, and storage service in Rust. The service handles certificate lifecycle management from multiple providers (Let's Encrypt, ZeroSSL, etc.) with automated renewal, secure storage (local, S3, Cloudflare R2), and zero-downtime certificate rotation.

## Problem Statement

Managing SSL/TLS certificates in production requires:

1. **Initial acquisition** - ACME protocol implementation, domain validation
2. **Renewal tracking** - Monitoring expiration, scheduling renewals
3. **Automatic renewal** - Proactive renewal before expiration
4. **Secure storage** - Local filesystem, S3, R2, secrets managers
5. **Distribution** - Notifying services of certificate updates
6. **Zero-downtime** - Hot-reload certificates without service interruption
7. **Multi-provider** - Fallback between ACME providers
8. **Observability** - Metrics, alerts, audit logging

## ACME Protocol Fundamentals

### What is ACME?

Automatic Certificate Management Environment (ACME) is an IETF standard (RFC 8555) for automating certificate issuance and renewal.

```
┌─────────────────────────────────────────────────────────────────┐
│                    ACME Protocol Flow                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Register Account                                             │
│     Client ──[POST /newAccount]──▶ CA                           │
│     Client ◀──[Account Location]───▶ CA                         │
│                                                                  │
│  2. Request Order (Certificate)                                  │
│     Client ──[POST /newOrder]────▶ CA                           │
│     Client ◀──[Order + Authorizations]─▶ CA                     │
│                                                                  │
│  3. Complete Challenges (Domain Validation)                      │
│     Client ──[POST /authz/{id}]──▶ CA                           │
│     Client ◀──[Challenge Token]──────▶ CA                       │
│     [Perform validation: HTTP-01, DNS-01, TLS-ALPN-01]          │
│     Client ──[POST /challenge]─────▶ CA                         │
│     Client ◀──[Valid]────────────────▶ CA                       │
│                                                                  │
│  4. Finalize Order                                               │
│     Client ──[POST /finalize]────▶ CA                           │
│     Client ◀──[Certificate]──────────▶ CA                       │
│                                                                  │
│  5. Renew (when needed)                                          │
│     [Repeat from step 2 with new order]                         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Challenge Types

| Challenge | Description | Use Case |
|-----------|-------------|----------|
| HTTP-01 | Place token at `/.well-known/acme-challenge/` | Web servers, load balancers |
| DNS-01 | Add TXT record `_acme-challenge.{domain}` | Wildcard certs, no HTTP access |
| TLS-ALPN-01 | TLS handshake with ALPN extension | Specialized servers |

### ACME Providers

| Provider | Rate Limits | Wildcard | DNS Providers | Notes |
|----------|-------------|----------|---------------|-------|
| Let's Encrypt | 50 certs/domain/week | Yes | Via acme-dns | Most popular, free |
| ZeroSSL | 3 certs/domain/day | Yes | Built-in | 90-day certs |
| Buypass | 5 certs/domain/week | No | No | 180-day certs |
| Google Trust | Varies | Yes | GCP only | GCP integrated |
| Stripe | Varies | Yes | Limited | Via Stripe Certs |

## Production Requirements

### 1. Certificate Acquisition

```
┌─────────────────────────────────────────────────────────────┐
│              Certificate Acquisition Flow                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐                                           │
│  │ Config Load  │                                           │
│  └──────┬───────┘                                           │
│         │                                                    │
│         ▼                                                    │
│  ┌──────────────┐    ┌──────────────┐                       │
│  │ Check Cache  │───▶│ Already Have │                       │
│  │ & Validate   │    │ Valid Cert?  │                       │
│  └──────────────┘    └──────┬───────┘                       │
│         │                   │ Yes                           │
│         │ No                │                               │
│         │                   ▼                               │
│         │           ┌──────────────┐                       │
│         │           │ Return Cert  │                       │
│         │           └──────────────┘                       │
│         ▼                                                    │
│  ┌──────────────┐                                           │
│  │ Select CA    │                                           │
│  │ (Provider)   │                                           │
│  └──────┬───────┘                                           │
│         │                                                    │
│         ▼                                                    │
│  ┌──────────────┐    ┌──────────────┐                       │
│  │ HTTP-01      │    │ DNS-01       │                       │
│  │ Challenge    │    │ Challenge    │                       │
│  └──────────────┘    └──────────────┘                       │
│         │                   │                                │
│         └─────────┬─────────┘                                │
│                   │                                          │
│                   ▼                                          │
│          ┌────────────────┐                                 │
│          │ Issue Certificate │                               │
│          └────────┬───────┘                                 │
│                   │                                          │
│                   ▼                                          │
│          ┌────────────────┐                                 │
│          │ Store & Return │                                 │
│          └────────────────┘                                 │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 2. Certificate Renewal Strategy

```rust
// Renewal timing considerations:
// - Let's Encrypt: 90-day certificates
// - Recommended renewal: 30 days before expiration
// - Staggered renewal for multiple domains
// - Exponential backoff on failures

Renewal Schedule:
├── T-30 days: First renewal attempt
├── T-25 days: Retry with backoff
├── T-20 days: Try alternate provider
├── T-15 days: Alert operations team
├── T-7 days:  Emergency mode (hourly attempts)
└── T-0 days:  Certificate expired (incident)
```

### 3. Storage Backends

| Backend | Use Case | Pros | Cons |
|---------|----------|------|------|
| Local Filesystem | Single server | Simple, fast | No distribution |
| S3 | Multi-region | Durable, versioned | Latency, cost |
| Cloudflare R2 | Edge distribution | No egress fees | Newer service |
| Kubernetes Secrets | K8s deployments | Native integration | Size limits |
| HashiCorp Vault | Centralized secrets | Audit, rotation | Operational overhead |
| AWS Secrets Manager | AWS deployments | Managed, rotated | Cost, vendor lock-in |

### 4. Certificate Distribution

```
┌─────────────────────────────────────────────────────────────┐
│              Certificate Distribution                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────┐                                        │
│  │ Cert Manager    │                                        │
│  │ Service         │                                        │
│  └────────┬────────┘                                        │
│           │                                                  │
│     ┌─────┼─────┬──────────┐                                │
│     │     │     │          │                                │
│     ▼     ▼     ▼          ▼                                │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐                       │
│  │Web-1 │ │Web-2 │ │API-1 │ │API-2 │                       │
│  │      │ │      │ │      │ │      │                       │
│  │[cert]│ │[cert]│ │[cert]│ │[cert]│                       │
│  └──────┘ └──────┘ └──────┘ └──────┘                       │
│                                                              │
│  Distribution Methods:                                       │
│  1. File watcher (inotify)                                   │
│  2. S3 event notifications                                   │
│  3. Pub/Sub (SNS/SQS, Redis)                                 │
│  4. gRPC stream                                              │
│  5. Kubernetes webhook                                       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Rust Ecosystem for ACME

### Core ACME Crates

| Crate | Purpose | Maturity |
|-------|---------|----------|
| `acme2` | Full ACME v2 client | Production |
| `rustls-acme` | rustls + ACME integration | Production |
| `le-acme` | Let's Encrypt specific | Beta |
| `acme-client` | Generic ACME client | Beta |
| `tokio-rustls` | Async TLS with rustls | Production |
| `rcgen` | Certificate generation | Production |
| `x509-parser` | X.509 parsing | Production |

### Cloud Storage Crates

| Crate | Purpose |
|-------|---------|
| `aws-sdk-s3` | AWS S3 client |
| `aws-config` | AWS configuration |
| `cloudflare` | Cloudflare API client |
| `serde-json` | JSON serialization |
| `tokio-retry` | Retry logic for API calls |

### Recommended Production Stack

```
┌────────────────────────────────────────────────────────────┐
│           Recommended Auto-SSL Stack                        │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ACME Client:       acme2 + rustls-acme                    │
│  TLS Library:       rustls (via tokio-rustls)              │
│  Async Runtime:     tokio (full features)                  │
│  Web Framework:     axum (for HTTP-01 challenges)          │
│  S3 Client:         aws-sdk-s3                             │
│  Cloudflare:        cloudflare-rust                        │
│  Storage:           Local + S3/R2 (dual-write)             │
│  Queue:             Redis/Tokio channels for renewal       │
│  Observability:     tracing + metrics-exporter             │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

## Production Architecture Patterns

### Pattern 1: Embedded ACME Handler

```
┌────────────────────────────────────────────────────────────┐
│                    Application Process                      │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Auto-SSL Manager                        │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐     │  │
│  │  │ ACME       │  │ Renewal    │  │ Storage    │     │  │
│  │  │ Client     │  │ Scheduler  │  │ Backend    │     │  │
│  │  └────────────┘  └────────────┘  └────────────┘     │  │
│  └──────────────────────────────────────────────────────┘  │
│                            │                                │
│                            ▼                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              HTTPS Server (rustls)                   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
└────────────────────────────────────────────────────────────┘

Use Case: Single binary, self-contained
Pros: Simple deployment, no external dependencies
Cons: Tied to application lifecycle
```

### Pattern 2: Sidecar Certificate Manager

```
┌────────────────────────────────────────────────────────────┐
│                    Kubernetes Pod                           │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────┐         ┌─────────────────┐           │
│  │  App Container  │         │  Auto-SSL       │           │
│  │                 │◀───────▶│  Sidecar        │           │
│  │  [reads certs   │  Unix   │  [ACME +        │           │
│  │   from shared   │  Socket │   Renewal +     │           │
│  │   volume]       │         │   Storage]      │           │
│  └────────┬────────┘         └────────┬────────┘           │
│           │                           │                     │
│           │    ┌─────────────────┐   │                     │
│  └────────┼────▶│  Shared Volume  │◀──┘                     │
│           │    │  /etc/ssl/certs │                         │
│           │    └─────────────────┘                         │
│           │                                                 │
└───────────│─────────────────────────────────────────────────┘
            │
            ▼
    [Application reads certs
     with hot-reload support]

Use Case: Kubernetes, container orchestration
Pros: Separation of concerns, reusable
Cons: K8s-specific, volume management
```

### Pattern 3: Centralized Certificate Service

```
┌─────────────────────────────────────────────────────────────────┐
│                    Certificate Service Cluster                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                        │
│  │ Auto-SSL│  │ Auto-SSL│  │ Auto-SSL│                        │
│  │ Node 1  │  │ Node 2  │  │ Node 3  │                        │
│  └────┬────┘  └────┬────┘  └────┬────┘                        │
│       │           │           │                                 │
│       └───────────┼───────────┘                                 │
│                   │                                             │
│          ┌────────▼────────┐                                   │
│          │  Coordination   │                                   │
│          │  (Redis/etcd)   │                                   │
│          └────────┬────────┘                                   │
│                   │                                             │
│       ┌───────────┼───────────┐                                │
│       │           │           │                                 │
│       ▼           ▼           ▼                                 │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐                          │
│  │  Local  │ │   S3    │ │Cloudflare│                          │
│  │ Storage │ │ Storage │ │  R2     │                          │
│  └─────────┘ └─────────┘ └─────────┘                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                    │
         ▼                    ▼                    ▼
  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
  │  Service A  │     │  Service B  │     │  Service C  │
  │  (HTTPS)    │     │  (HTTPS)    │     │  (HTTPS)    │
  └─────────────┘     └─────────────┘     └─────────────┘

Use Case: Multi-service organizations, central platform team
Pros: Centralized management, cost efficient
Cons: Single point of failure (needs HA)
```

### Pattern 4: DNS Provider Integration

```
┌─────────────────────────────────────────────────────────────┐
│              DNS-01 Challenge Flow                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Auto-SSL Service                                           │
│       │                                                      │
│       │ 1. Request certificate                               │
│       │    (wildcard: *.example.com)                        │
│       ▼                                                      │
│  ┌─────────────┐                                            │
│  │ ACME Server │ 2. Return DNS challenge                    │
│  │ (Let's Enc) │    token + DNS name                        │
│  └──────┬──────┘                                            │
│         │                                                    │
│         │ 3. Create TXT record                               │
│         │    _acme-challenge.example.com                    │
│         ▼                                                    │
│  ┌─────────────┐                                            │
│  │ DNS Provider│ (Cloudflare, Route53, etc.)                │
│  └──────┬──────┘                                            │
│         │                                                    │
│         │ 4. CA validates DNS record                         │
│         │                                                    │
│         │ 5. Issue certificate                               │
│         ▼                                                    │
│  ┌─────────────┐                                            │
│  │ Certificate │                                            │
│  └─────────────┘                                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Considerations

### 1. Challenge Server for HTTP-01

```rust
// HTTP-01 requires serving challenge at:
// http://<domain>/.well-known/acme-challenge/<token>

// Options:
// a) Bind to port 80 directly
// b) Use existing reverse proxy (nginx, traefik)
// c) Use DNS-01 instead (no port 80 needed)
```

### 2. Rate Limiting Awareness

| Limit Type | Let's Encrypt | Mitigation |
|------------|---------------|------------|
| Certificates | 50/domain/week | Cache aggressively |
| Failures | 50/hour | Backoff on errors |
| Registrations | 10/account/week | Reuse accounts |
| Duplicate | 5/week | Check existing certs |

### 3. Account Key Management

```rust
// ACME account keys should be:
// - Generated once and persisted
// - Stored securely (encrypted at rest)
// - Backed up for recovery
// - Rotated periodically (yearly)

// Account key storage options:
// 1. Local encrypted file
// 2. Cloud KMS (AWS KMS, GCP KMS)
// 3. HSM (for high-security deployments)
```

### 4. Certificate Storage Format

```
Storage Structure (S3/R2):
s3://bucket/certificates/
├── {domain}/
│   ├── cert.pem          # Full certificate chain
│   ├── privkey.pem       # Private key (encrypted)
│   ├── chain.pem         # Intermediate chain
│   ├── metadata.json     # Expiry, provider, etc.
│   └── {timestamp}/      # Historical versions
│       ├── cert.pem
│       └── privkey.pem
```

## Security Best Practices

### Private Key Protection

| Practice | Implementation |
|----------|----------------|
| Encryption at rest | AES-256-GCM with KMS |
| Access control | IAM policies, least privilege |
| Audit logging | All key access logged |
| Memory protection | Zeroize after use |
| Transmission | TLS 1.3 for all transfers |

### Account Security

```rust
// ACME account hardening:
// 1. Use separate accounts per environment
// 2. Register with dedicated email (not personal)
// 3. Enable account key rotation
// 4. Monitor for unauthorized orders
// 5. Implement webhooks for notifications
```

### Certificate Validation

```rust
// Always validate certificates before use:
// 1. Check expiration (not_after > now + buffer)
// 2. Verify chain of trust
// 3. Validate domain matches
// 4. Check revocation status (OCSP/CRL)
// 5. Verify key type and size
```

## Monitoring and Observability

### Key Metrics

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `cert_expiry_days{domain}` | Days until expiration | < 14 days |
| `acme_requests_total{status}` | ACME request count | Error rate > 1% |
| `acme_request_duration_seconds` | ACME operation latency | p99 > 30s |
| `storage_write_duration_seconds` | Storage operation latency | p99 > 5s |
| `renewal_success_total` | Successful renewals | 0 in 24h |
| `renewal_failure_total` | Failed renewals | Any |
| `challenge_success_total` | Challenge completions | Error rate > 5% |

### Alerting Rules

```yaml
# Prometheus alerting rules example
groups:
  - name: ssl-certificates
    rules:
      - alert: CertificateExpiringSoon
        expr: cert_expiry_days < 14
        for: 1h
        labels:
          severity: warning
        annotations:
          summary: "Certificate for {{ $labels.domain }} expires in {{ $value }} days"
      
      - alert: CertificateExpired
        expr: cert_expiry_days < 0
        labels:
          severity: critical
        annotations:
          summary: "Certificate for {{ $labels.domain }} has expired"
      
      - alert: RenewalFailure
        expr: renewal_failure_total > 0
        for: 30m
        labels:
          severity: warning
        annotations:
          summary: "Certificate renewal failed for {{ $labels.domain }}"
```

## Testing Strategies

### Unit Tests

```rust
// Test certificate parsing
// Test expiry calculation
// Test storage backend mocks
// Test ACME challenge generation
```

### Integration Tests

```rust
// Test against staging Let's Encrypt
// Test DNS provider integrations
// Test S3/R2 storage
// Test full renewal cycle
```

### End-to-End Tests

```rust
// Spin up test domain
// Acquire real certificate
// Deploy to test server
// Verify HTTPS works
// Trigger renewal
// Verify zero-downtime rotation
```

### Staging Environment

```rust
// Let's Encrypt provides staging CA:
// - https://acme-staging-v02.api.letsencrypt.org/directory
// - No rate limits
// - Untrusted certificates (for testing only)
// - Use for all integration tests
```

## Migration Path

### Phase 1: Manual Certificate Management

```
- Document current certificate process
- Inventory all certificates and expiration dates
- Set up monitoring for expiration
```

### Phase 2: Semi-Automated

```
- Deploy auto-SSL in staging
- Test HTTP-01 and DNS-01 challenges
- Configure S3/R2 storage
- Run parallel to manual process
```

### Phase 3: Automated Renewal

```
- Enable automatic renewal in staging
- Verify renewal notifications work
- Test zero-downtime rotation
- Document runbooks
```

### Phase 4: Production Rollout

```
- Deploy to production (read-only initially)
- Enable for non-critical domains first
- Gradually expand coverage
- Decommission manual processes
```

## Related Explorations

- `../rust-mtls/exploration.md` - Mutual TLS implementation
- `../utm-dev-production/security-signing-exploration.md` - Security patterns

## Next Steps

1. Create `rust-revision.md` with complete implementation
2. Create working example projects:
   - Embedded ACME handler
   - Standalone certificate manager
   - Storage backends (S3, R2, local)
3. Create deep-dive documents:
   - DNS provider integration
   - Kubernetes deployment
   - High-availability setup

