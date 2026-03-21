---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/rust-mtls
repository: N/A - Greenfield exploration
explored_at: 2026-03-21
---

# Mutual mTLS Production Exploration with Rust

## Overview

This exploration details the implementation of mutual TLS (mTLS) authentication in production environments using Rust. mTLS provides bidirectional authentication where both client and server verify each other's certificates, offering stronger security than traditional TLS (server-only authentication).

## What is mTLS?

Mutual TLS extends standard TLS by requiring:
1. **Server presents certificate** → Client verifies server identity (standard TLS)
2. **Client presents certificate** → Server verifies client identity (mTLS addition)
3. **Both parties prove possession of private keys** → Mutual authentication

```
┌─────────────────────────────────────────────────────────────────┐
│                    Standard TLS vs mTLS                          │
├─────────────────────────────────────────────────────────────────┤
│ Standard TLS:                                                    │
│   Client ──[verify server cert]──▶ Server                       │
│                                                                      │
│ mTLS:                                                            │
│   Client ◀──[mutual cert verification]──▶ Server                │
│   (bidirectional trust)                                          │
└─────────────────────────────────────────────────────────────────┘
```

## Use Cases for mTLS

| Use Case | Why mTLS |
|----------|----------|
| Service-to-service authentication | Zero-trust microservices architecture |
| API client authentication | Replace API keys with cryptographic identity |
| IoT device authentication | Hardware-backed certificate storage |
| Internal service mesh | Istio, Linkerd service-to-service security |
| B2B integrations | Strong partner authentication |
| Admin/ops access | Privileged access without passwords |

## Production Requirements Checklist

### 1. Certificate Authority (CA) Setup

```
┌─────────────────────────────────────────────────────────────┐
│                    PKI Hierarchy                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────┐                                        │
│  │  Root CA        │ ← Offline, highly protected            │
│  │  (self-signed)  │                                        │
│  └────────┬────────┘                                        │
│           │ issues & signs                                   │
│           ▼                                                  │
│  ┌─────────────────┐                                        │
│  │  Intermediate   │ ← Issues end-entity certs              │
│  │  CA             │                                        │
│  └────────┬────────┘                                        │
│           │ issues & signs                                   │
│           ▼                                                  │
│  ┌─────────────────┐         ┌─────────────────┐           │
│  │  Server Certs   │         │  Client Certs   │           │
│  │  (end-entity)   │         │  (end-entity)   │           │
│  └─────────────────┘         └─────────────────┘           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Production considerations:**
- Root CA should be offline/air-gapped
- Intermediate CA handles day-to-day issuance
- Certificate revocation lists (CRL) or OCSP for revocation
- Automated certificate rotation before expiration

### 2. Certificate Lifecycle Management

| Phase | Requirements |
|-------|--------------|
| Generation | Secure private key generation (HSM/backed) |
| Distribution | Secure delivery to clients/services |
| Storage | Protected storage (filesystem, vault, HSM) |
| Rotation | Automated renewal before expiration |
| Revocation | CRL/OCSP for compromised certs |
| Audit | Logging and tracking of all cert operations |

### 3. Server-Side Requirements

```rust
// Server must:
// 1. Present its own certificate to clients
// 2. Request client certificate during handshake
// 3. Verify client certificate against trusted CA
// 4. Extract identity from client cert for authorization
// 5. Handle certificate expiration/revocation
```

### 4. Client-Side Requirements

```rust
// Client must:
// 1. Store client certificate and private key securely
// 2. Present certificate when server requests it
// 3. Verify server certificate (as in standard TLS)
// 4. Handle certificate rotation transparently
```

## Rust Ecosystem for mTLS

### Core TLS/SSL Crates

| Crate | Purpose | Maturity |
|-------|---------|----------|
| `tokio-rustls` | Async TLS with rustls | Production |
| `rustls` | Modern TLS library (no OpenSSL) | Production |
| `tokio-openssl` | Async TLS with OpenSSL | Production |
| `openssl` | OpenSSL bindings | Production |
| `native-tls` | OS-native TLS backend | Production |
| `hyper-rustls` | HTTP client with rustls | Production |
| `axum-server` | AXUM server with TLS | Production |
| `tower` | Service middleware for TLS | Production |

### Certificate Management

| Crate | Purpose |
|-------|---------|
| `rcgen` | Certificate generation (testing) |
| `x509-parser` | X.509 certificate parsing |
| `rustls-pemfile` | PEM file parsing |
| `rustls-native-certs` | Load system CA certs |
| `webpki` | Certificate verification |
| `step-ca` | Certificate authority (external) |

### Recommended Production Stack

```
┌────────────────────────────────────────────────────────────┐
│              Recommended Rust mTLS Stack                    │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Async Runtime:     tokio (full features)                  │
│  TLS Library:       rustls (via tokio-rustls)              │
│                     - Pure Rust, no OpenSSL dependency     │
│                     - Audited, memory-safe                 │
│                     - Good performance                     │
│  Web Framework:     axum + axum-server                     │
│  HTTP Client:       reqwest (with rustls)                  │
│  Middleware:        tower + tower-http                     │
│  Cert Parsing:      x509-parser                            │
│  Cert Generation:   rcgen (testing/dev only)               │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

## Production Architecture Patterns

### Pattern 1: Direct mTLS Server

```
                              ┌─────────────────────────┐
                              │    Rust mTLS Server     │
                              │  ┌───────────────────┐  │
Client with cert ────────────▶│  │  TLS Acceptor     │  │
                              │  │  (requires cert)  │  │
                              │  └─────────┬─────────┘  │
                              │            │            │
                              │  ┌─────────▼─────────┐  │
                              │  │  Cert Validator   │  │
                              │  │  (CN/SAN extract) │  │
                              │  └─────────┬─────────┘  │
                              │            │            │
                              │  ┌─────────▼─────────┐  │
                              │  │  AuthZ Handler    │  │
                              │  └───────────────────┘  │
                              └─────────────────────────┘
```

### Pattern 2: mTLS with Reverse Proxy

```
                    ┌──────────────┐
                    │   NGINX /    │
                    │  Envoy Proxy │
                    │  (mTLS term) │
                    └──────┬───────┘
                           │
                           │ Internal (plain/mTLS)
                           │
                    ┌──────▼───────┐
                    │  Rust App    │
                    │  (simplified)│
                    └──────────────┘

// Proxy handles mTLS termination
// Forwards client cert via headers (X-Client-Cert, X-Forwarded-Client-Cert)
// Rust app validates forwarded cert headers
```

### Pattern 3: Service Mesh mTLS (Istio/Linkerd)

```
┌─────────────────────────────────────────────────────────────┐
│  Sidecar proxy handles ALL mTLS                              │
│                                                              │
│  App ◀──[plaintext]──▶ Sidecar ◀──[mTLS]──▶ Other Sidecars │
│                                                              │
│  Rust app: zero mTLS code                                    │
│  Mesh: handles certs, rotation, verification                 │
└─────────────────────────────────────────────────────────────┘
```

### Pattern 4: mTLS Microservices

```
┌──────────────────────────────────────────────────────────────┐
│                    mTLS Service Mesh                          │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────┐         ┌──────────┐         ┌──────────┐     │
│  │ Service  │◀─mTLS──▶│ Service  │◀─mTLS──▶│ Service  │     │
│  │    A     │         │    B     │         │    C     │     │
│  │ (client) │         │(service) │         │(service) │     │
│  └────┬─────┘         └────┬─────┘         └────┬─────┘     │
│       │                    │                    │           │
│       └────────────────────┴────────────────────┘           │
│                            │                                │
│                   ┌────────▼────────┐                       │
│                   │  Internal CA    │                       │
│                   │  (cert issuer)  │                       │
│                   └─────────────────┘                       │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

## Implementation Considerations

### 1. Certificate Verification Depth

```rust
// How deep should certificate chain verification go?
// Options:
// - VerifyOnly: Verify chain to trusted root
// - VerifyAndRequireClientCert: Above + require client cert
// - VerifyWithCRL: Above + check revocation lists
// - VerifyWithOCSP: Above + OCSP stapling check
```

### 2. Client Certificate Modes

| Mode | Description | Use Case |
|------|-------------|---------|
| `Required` | Client MUST present cert | High-security internal APIs |
| `Optional` | Client MAY present cert | Mixed client environments |
| `None` | No client cert requested | Standard TLS |

### 3. Identity Extraction

```
Certificate Subject Fields:
├── CN (Common Name) → Primary identifier
├── O (Organization) → Tenant/Org ID
├── OU (Org Unit) → Department/Team
├── emailAddress → Contact
└── SAN (Subject Alternative Names)
    ├── DNS names
    ├── IP addresses
    ├── URIs (spiffe://)
    └── Email addresses
```

### 4. Authorization Strategies

```rust
// After extracting identity from cert:
// 1. CN-based authorization
// 2. SAN URI-based (SPIFFE IDs)
// 3. Certificate fingerprint allowlist
// 4. OAuth-style scopes in SAN
// 5. RBAC lookup by cert identity
```

## Security Best Practices

### Certificate Requirements

| Requirement | Recommendation |
|-------------|----------------|
| Key Size | RSA 3072+ or ECDSA P-256+ |
| Hash Algorithm | SHA-256 or better |
| Validity Period | ≤ 1 year for services |
| Key Storage | HSM or secure enclave |
| Revocation | OCSP stapling preferred |

### Handshake Hardening

```rust
// Recommended TLS configuration:
// - TLS 1.3 only (disable 1.2 and below)
// - Strong cipher suites only
// - ALPN for protocol negotiation
// - Session tickets disabled (forward secrecy)
// - Certificate compression enabled
```

### Common Vulnerabilities to Avoid

| Vulnerability | Mitigation |
|---------------|------------|
| Missing client cert verification | Always set `ClientAuth::Required` |
| Not checking certificate chain | Use `WebPkiClientVerifier` |
| Ignoring revocation status | Implement CRL/OCSP checks |
| Storing private keys insecurely | Use vault/HSM |
| No certificate rotation | Implement automated renewal |
| Weak cipher suites | Use rustls safe defaults |
| Not validating SAN fields | Parse and validate all identities |

## Testing Strategies

### 1. Unit Testing

```rust
// Test certificate parsing
// Test identity extraction
// Test authorization logic
// Mock TLS connections
```

### 2. Integration Testing

```rust
// Create test CA and certificates
// Spin up test server with mTLS
// Create test clients with valid/invalid certs
// Verify handshake behavior
// Test certificate expiration scenarios
```

### 3. End-to-End Testing

```rust
// Full mTLS handshake in staging
// Load testing with mTLS overhead
// Certificate rotation simulation
// Failure mode testing
```

### Test Certificate Generation

```rust
// Use rcgen for test certificates:
// - Generate test CA
// - Generate server cert signed by test CA
// - Generate client cert signed by test CA
// - Generate expired cert
// - Generate revoked cert
// - Generate cert with missing SAN
```

## Deployment Considerations

### Container Deployments

```yaml
# Kubernetes mTLS deployment considerations:
# 1. Mount certificates as volumes (from secrets)
# 2. Use cert-manager for automated provisioning
# 3. Configure init containers for cert validation
# 4. Set up sidecar for certificate rotation
# 5. Configure proper file permissions (600)
```

### Certificate Storage Options

| Storage | Use Case | Pros | Cons |
|---------|----------|------|------|
| Filesystem | Simple deployments | Easy to implement | Rotation complexity |
| Kubernetes Secrets | K8s deployments | Native integration | Base64 encoded |
| HashiCorp Vault | Centralized secrets | Audit, rotation | Additional dependency |
| AWS Secrets Manager | AWS deployments | Managed service | Vendor lock-in |
| HSM | High security | Hardware protection | Cost, complexity |

### Monitoring and Observability

```rust
// Metrics to track:
// - mTLS handshake success/failure rate
// - Certificate expiration (days remaining)
// - Client identity distribution
// - Revocation check failures
// - TLS version distribution
// - Cipher suite distribution

// Alerts to configure:
// - Certificate expiring within 30 days
// - Spike in handshake failures
// - Unknown client certificates
// - Revoked certificate usage attempts
```

## Migration Path to mTLS

### Phase 1: Server-Side TLS Only
```
- Deploy server with TLS
- Distribute CA cert to clients
- Verify server certificates working
```

### Phase 2: Optional Client Certs
```
- Configure server to accept optional client certs
- Issue client certs to early adopters
- Log client cert presence/absence
- Build authorization logic
```

### Phase 3: Required Client Certs
```
- Switch to required client cert mode
- All clients must present valid certs
- Monitor for any failures
- Have rollback plan ready
```

### Phase 4: Operational Maturity
```
- Automated certificate rotation
- Revocation checking enabled
- Full monitoring and alerting
- Documentation and runbooks
```

## Code Examples Location

This exploration will be followed by a `rust-revision.md` containing:
- Complete working mTLS server implementation
- Complete working mTLS client implementation
- Test utilities for certificate generation
- Integration test examples
- Production deployment configurations

## Related Explorations

- `../src.process-compose/rust-revision.md` - Process orchestration patterns
- `../utm-dev-production/security-signing-exploration.md` - Security patterns
- `../protocols/mpp-rs/exploration.md` - Protocol implementations

## Next Steps

1. Create `rust-revision.md` with complete implementation details
2. Create example projects demonstrating:
   - Basic mTLS server
   - Basic mTLS client
   - Certificate generation utilities
   - Integration test setup
3. Create deep-dive documents for:
   - Certificate authority setup
   - Kubernetes mTLS deployment
   - Service mesh integration
