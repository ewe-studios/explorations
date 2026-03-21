# Certificate Management for mTLS

## Overview

This document covers certificate authority setup, certificate generation, lifecycle management, and rotation strategies for production mTLS deployments.

## Certificate Authority Setup

### Root CA Creation

```bash
# Generate Root CA private key (4096-bit RSA)
openssl genrsa -out ca.key 4096

# Set restrictive permissions
chmod 600 ca.key

# Generate Root CA certificate (10 year validity)
openssl req -x509 -new -nodes -key ca.key -sha256 \
  -days 3650 \
  -out ca.crt \
  -subj "/C=US/ST=California/L=San Francisco/O=Example Inc/OU=Security/CN=Example Root CA"
```

### Intermediate CA Creation

```bash
# Generate Intermediate CA private key
openssl genrsa -out intermediate.key 4096
chmod 600 intermediate.key

# Generate Certificate Signing Request (CSR)
openssl req -new -key intermediate.key \
  -out intermediate.csr \
  -subj "/C=US/ST=California/L=San Francisco/O=Example Inc/OU=Security/CN=Example Intermediate CA"

# Create CA extensions config
cat > ca_ext.cnf <<EOF
basicConstraints = critical, CA:TRUE, pathlen:0
keyUsage = critical, digitalSignature, cRLSign, keyCertSign
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always, issuer
EOF

# Sign the intermediate certificate
openssl x509 -req -in intermediate.csr \
  -CA ca.crt -CAkey ca.key -CAcreateserial \
  -out intermediate.crt \
  -days 1825 \
  -extfile ca_ext.cnf \
  -sha256
```

### CA Directory Structure

```
pki/
├── root/
│   ├── ca.key              # Root CA private key (OFFLINE)
│   ├── ca.crt              # Root CA certificate
│   └── ca.srl              # Serial number file
├── intermediate/
│   ├── intermediate.key    # Intermediate CA private key
│   ├── intermediate.crt    # Intermediate CA certificate
│   └── intermediate.srl    # Serial number file
├── certs/                  # Issued certificates
│   ├── api-gateway.crt
│   ├── user-service.crt
│   └── order-service.crt
├── private/                # Private keys (restricted access)
│   ├── api-gateway.key
│   ├── user-service.key
│   └── order-service.key
└── crl/                    # Certificate Revocation Lists
    ├── root.crl
    └── intermediate.crl
```

## Service Certificate Generation

### OpenSSL Configuration for Service Certs

```bash
# Server certificate extensions
cat > server_ext.cnf <<EOF
basicConstraints = CA:FALSE
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth, clientAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = api-gateway
DNS.2 = api-gateway.default.svc.cluster.local
DNS.3 = api-gateway.example.com
URI.1 = spiffe://example.com/ns/default/sa/api-gateway
EOF

# Client certificate extensions (for client auth)
cat > client_ext.cnf <<EOF
basicConstraints = CA:FALSE
keyUsage = critical, digitalSignature
extendedKeyUsage = clientAuth
subjectAltName = @alt_names

[alt_names]
CN = api-gateway-client
URI.1 = spiffe://example.com/ns/default/sa/api-gateway-client
EOF
```

### Generate Service Certificate

```bash
#!/bin/bash
# generate-service-cert.sh

SERVICE_NAME=$1
DAYS=${2:-365}

# Generate private key
openssl genrsa -out "private/${SERVICE_NAME}.key" 2048
chmod 600 "private/${SERVICE_NAME}.key"

# Generate CSR
openssl req -new -key "private/${SERVICE_NAME}.key" \
  -out "certs/${SERVICE_NAME}.csr" \
  -subj "/C=US/ST=California/O=Example Inc/CN=${SERVICE_NAME}"

# Sign certificate with intermediate CA
openssl x509 -req -in "certs/${SERVICE_NAME}.csr" \
  -CA intermediate/intermediate.crt \
  -CAkey intermediate/intermediate.key \
  -CAcreateserial \
  -out "certs/${SERVICE_NAME}.crt" \
  -days $DAYS \
  -extfile server_ext.cnf \
  -sha256

echo "Certificate generated: certs/${SERVICE_NAME}.crt"
echo "Valid for: $DAYS days"
```

### Rust Certificate Generation (rcgen)

```rust
// Use rcgen for programmatic certificate generation
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType};
use time::{Duration, OffsetDateTime};

fn generate_service_cert(service_name: &str, days: i64) -> Result<(String, String)> {
    let mut params = CertificateParams::default();

    // Set subject
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CountryName, "US".to_string());
    dn.push(DnType::OrganizationName, "Example Inc".to_string());
    dn.push(DnType::CommonName, service_name.to_string());
    params.distinguished_name = dn;

    // Set validity
    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + Duration::days(days);

    // Set SANs
    params.subject_alt_names = vec![
        rcgen::SanType::DnsName(service_name.to_string()),
        rcgen::SanType::DnsName(format!("{}.default.svc.cluster.local", service_name)),
        rcgen::SanType::Uri(
            format!("spiffe://example.com/ns/default/sa/{}", service_name)
                .parse().unwrap()
        ),
    ];

    // Generate certificate
    let cert = Certificate::from_params(params)?;
    let cert_pem = cert.serialize_pem()?;
    let key_pem = cert.serialize_private_key_pem();

    Ok((cert_pem, key_pem))
}
```

## Certificate Lifecycle Management

### Automated Rotation with Kubernetes Cert-Manager

```yaml
# cert-manager ClusterIssuer
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: mtls-issuer
spec:
  ca:
    secretName: mtls-ca-secret
---
# Certificate resource
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: api-gateway-cert
  namespace: default
spec:
  secretName: api-gateway-tls
  duration: 720h  # 30 days
  renewBefore: 168h  # Renew 1 week before expiry
  issuerRef:
    name: mtls-issuer
    kind: ClusterIssuer
  commonName: api-gateway.default.svc.cluster.local
  dnsNames:
    - api-gateway
    - api-gateway.default.svc.cluster.local
    - api-gateway.example.com
  uriSANs:
    - spiffe://example.com/ns/default/sa/api-gateway
  usages:
    - digital signature
    - key encipherment
    - server auth
    - client auth
```

### Certificate Renewal Script

```bash
#!/bin/bash
# renew-cert.sh - Check and renew certificates nearing expiry

CERT_DIR="certs"
WARNING_DAYS=14
CRITICAL_DAYS=7

check_cert_expiry() {
    local cert=$1
    local expiry=$(openssl x509 -in "$cert" -noout -enddate | cut -d= -f2)
    local expiry_epoch=$(date -d "$expiry" +%s)
    local now_epoch=$(date +%s)
    local days_left=$(( (expiry_epoch - now_epoch) / 86400 ))

    echo "$cert: expires in $days_left days ($expiry)"

    if [ $days_left -lt $CRITICAL_DAYS ]; then
        echo "  [CRITICAL] Certificate needs immediate renewal"
        return 1
    elif [ $days_left -lt $WARNING_DAYS ]; then
        echo "  [WARNING] Certificate should be renewed soon"
        return 2
    else
        echo "  [OK] Certificate is valid"
        return 0
    fi
}

# Check all certificates
for cert in $CERT_DIR/*.crt; do
    check_cert_expiry "$cert"
    echo ""
done
```

## Certificate Revocation

### CRL Generation

```bash
# Create index file and serial
touch index.txt
echo "01" > crlnumber

# Create OpenSSL config for CRL
cat > openssl_crl.cnf <<EOF
[ca]
default_ca = CA_default

[CA_default]
dir = ./pki
database = $dir/index.txt
crlnumber = $dir/crlnumber
default_crl_days = 30
default_md = sha256
crl_extensions = crl_ext

[crl_ext]
authorityKeyIdentifier = keyid:always
EOF

# Revoke a certificate
openssl ca -config openssl_crl.cnf \
  -revoke certs/revoked-service.crt \
  -keyfile intermediate/intermediate.key \
  -cert intermediate/intermediate.crt

# Generate CRL
openssl ca -config openssl_crl.cnf \
  -gencrl \
  -keyfile intermediate/intermediate.key \
  -cert intermediate/intermediate.crt \
  -out crl/intermediate.crl
```

### OCSP Responder Setup

```rust
// Basic OCSP responder concept
use x509_parser::prelude::*;

struct OcspResponder {
    ca_cert: Certificate,
    ca_key: PrivateKey,
    certificate_map: HashMap<String, CertStatus>,
}

enum CertStatus {
    Good,
    Revoked { reason: CrlReason, time: OffsetDateTime },
    Unknown,
}

impl OcspResponder {
    fn handle_request(&self, cert_id: &OcspCertId) -> OcspResponse {
        match self.certificate_map.get(&cert_id.serial_number) {
            Some(status) => OcspResponse::successful(status.clone()),
            None => OcspResponse::unauthorized(),
        }
    }
}
```

## SPIFFE/SPIRE Integration

### SPIFFE ID Format

```
spiffe://<trust-domain>/ns/<namespace>/sa/<service-account>

Examples:
spiffe://example.com/ns/default/sa/api-gateway
spiffe://prod.acme.io/ns/production/sa/payment-service
spiffe://cluster.local/ns/kube-system/sa/coredns
```

### SPIRE Agent Configuration

```hcl
# SPIRE Agent config
agent {
  data_dir = "/var/lib/spire"
  log_level = "DEBUG"
  server_address = "spire-server"
  server_port = "8081"
  socket_path = "/run/spire/agent.sock"
  trust_bundle_path = "/var/lib/spire/bundle.crt"
  trust_domain = "example.com"
}

plugins NodeAttestor "k8s_sat" {
  plugin_data {
    cluster = "production"
  }
}

plugins WorkloadAttestor "k8s" {
  plugin_data {
    skip_kubelet_verification = true
  }
}
```

### Fetching SVIDs

```bash
# Fetch X.509 SVID using SPIRE CLI
spire-agent api fetch x509 -socketPath /run/spire/agent.sock -write /tmp/svid

# Output files:
# /tmp/svid/0.crt - Certificate chain
# /tmp/svid/0.key - Private key
# /tmp/svid/bundle.crt - Trust bundle
```

## Certificate Verification

### Manual Verification

```bash
# Verify certificate chain
openssl verify -CAfile ca.crt -untrusted intermediate.crt service.crt

# Check certificate details
openssl x509 -in service.crt -noout -text

# Verify certificate matches private key
openssl x509 -in service.crt -noout -modulus | md5sum
openssl rsa -in service.key -noout -modulus | md5sum
# Outputs should match

# Check certificate dates
openssl x509 -in service.crt -noout -dates

# Verify SAN entries
openssl x509 -in service.crt -noout -ext subjectAltName
```

### Automated Verification Script

```rust
use x509_parser::prelude::*;
use time::OffsetDateTime;

fn verify_certificate(cert_pem: &[u8], ca_pem: &[u8]) -> Result<CertValidation> {
    let cert = parse_x509_certificate(cert_pem)?;
    let ca = parse_x509_certificate(ca_pem)?;

    let mut errors = Vec::new();

    // Check validity period
    let now = OffsetDateTime::now_utc();
    if now < cert.1.validity().not_before {
        errors.push(ValidationError::NotYetValid);
    }
    if now > cert.1.validity().not_after {
        errors.push(ValidationError::Expired);
    }

    // Check signature
    if !cert.1.verify_signature(Some(&ca.1))? {
        errors.push(ValidationError::InvalidSignature);
    }

    // Check issuer
    if cert.1.issuer() != ca.1.subject() {
        errors.push(ValidationError::IssuerMismatch);
    }

    // Check key usage
    let key_usage = cert.1.key_usage()?;
    if !key_usage.contains(KeyUsage::DigitalSignature) {
        errors.push(ValidationError::KeyUsageViolation);
    }

    Ok(CertValidation {
        valid: errors.is_empty(),
        errors,
    })
}
```

## Best Practices

### Certificate Configuration

| Setting | Recommendation | Rationale |
|---------|---------------|-----------|
| Key Size | 2048+ bits (RSA), 256+ bits (EC) | Security |
| Hash Algorithm | SHA-256 or better | No known weaknesses |
| Certificate Lifetime | 7-30 days (services) | Reduces exposure window |
| CA Lifetime | 5-10 years (intermediate) | Balance security/ops |
| Key Permissions | 0600 (owner read/write only) | Prevent unauthorized access |

### Operational Checklist

- [ ] Root CA stored offline/in HSM
- [ ] Intermediate CA used for signing
- [ ] Certificates include SANs (DNS + SPIFFE URI)
- [ ] Automated rotation configured
- [ ] Monitoring for expiry (14d, 7d alerts)
- [ ] CRL/OCSP infrastructure in place
- [ ] Private keys never leave host
- [ ] Certificate inventory maintained
- [ ] Revocation procedure documented
- [ ] Backup CA infrastructure tested

## Troubleshooting

### Common Issues

| Error | Cause | Solution |
|-------|-------|----------|
| "unknown CA" | Missing intermediate in chain | Include full chain in cert file |
| "certificate has expired" | Expired cert | Renew immediately |
| "hostname mismatch" | SAN doesn't match | Regenerate with correct SANs |
| "key usage violation" | Wrong key usage | Regenerate with correct extensions |
| "signature verification failed" | Wrong CA or corrupted cert | Regenerate certificate |

### Debug Commands

```bash
# View full certificate chain
openssl crl2pkcs7 -nocrl -certfile chain.pem | openssl pkcs7 -print_certs -noout

# Test TLS connection
openssl s_client -connect host:port -CAfile ca.crt -cert client.crt -key client.key

# Check OCSP response
openssl ocsp -issuer intermediate.crt -cert service.crt -url http://ocsp.example.com
```
