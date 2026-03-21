# mTLS Troubleshooting Guide

## Overview

This document provides diagnostic techniques, common issues, and solutions for mTLS deployments.

## Diagnostic Tools

### OpenSSL Diagnostics

```bash
# Test basic TLS connection
openssl s_client -connect host:port -CAfile ca.crt

# Test with client certificate
openssl s_client -connect host:port \
  -CAfile ca.crt \
  -cert client.crt \
  -key client.key

# Verify certificate chain
openssl verify -CAfile ca.crt -untrusted intermediate.crt service.crt

# View certificate details
openssl x509 -in certificate.crt -noout -text

# Check certificate dates
openssl x509 -in certificate.crt -noout -dates
openssl x509 -in certificate.crt -noout -checkend 86400  # Expires in 24h?

# Verify certificate matches key
openssl x509 -in cert.crt -noout -modulus | openssl md5
openssl rsa -in key.key -noout -modulus | openssl md5
# Outputs should match

# Test SNI
openssl s_client -connect host:port -servername server.name

# Check OCSP response
openssl ocsp -issuer intermediate.crt \
  -cert service.crt \
  -url http://ocsp.example.com \
  -text
```

### curl Diagnostics

```bash
# Test with verbose output
curl -v https://host:port/

# Test with client certificate
curl -v --cert client.crt --key client.key --cacert ca.crt https://host:port/

# Test with certificate chain
curl -v --cert client.crt --key client.key --cert-chain intermediate.crt https://host:port/

# Skip certificate verification (debug only)
curl -v -k https://host:port/

# Show certificate info
curl -vI https://host:port/ 2>&1 | grep -A5 "Server certificate"
```

### Rust Debug Logging

```rust
use tracing_subscriber::{fmt, EnvFilter};

fn init_debug_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_thread_ids(true)
        .init();
}

// Set RUST_LOG=trace,hyper=debug,rustls=debug
```

## Common Issues

### Issue 1: "unknown CA" Error

**Symptoms:**
```
error:1416F086:SSL routines:tls_process_server_certificate:certificate verify failed
```

**Causes:**
- Missing intermediate certificate in chain
- Client doesn't trust the CA
- Wrong CA file path

**Solution:**
```bash
# Verify chain locally first
openssl verify -CAfile ca.crt -untrusted intermediate.crt server.crt

# Ensure server sends full chain
# Concatenate: cat server.crt intermediate.crt > chain.pem

# In Rust server config, load full chain:
let certs = load_certs(Path::new("chain.pem"))?;  // Not just server.crt
```

### Issue 2: "certificate has expired"

**Symptoms:**
```
error:14094415:SSL routines:ssl3_read_bytes:sslv3 alert certificate expired
```

**Causes:**
- Certificate actually expired
- System clock skew
- Wrong certificate deployed

**Solution:**
```bash
# Check expiry
openssl x509 -in cert.crt -noout -dates

# Check current time
date -u

# Check cert expiry in seconds
openssl x509 -in cert.crt -noout -checkend 0

# Automated fix with cert-manager
kubectl cert-manager renew certificate-name
```

### Issue 3: "hostname mismatch"

**Symptoms:**
```
error:1416F086:SSL routines:tls_process_server_certificate:certificate verify failed:certificate does not match host
```

**Causes:**
- SAN doesn't include requested hostname
- Using IP address when only DNS SAN present
- Wrong certificate deployed

**Solution:**
```bash
# Check SAN entries
openssl x509 -in cert.crt -noout -ext subjectAltName

# Regenerate with correct SANs
# Add to OpenSSL config:
# [alt_names]
# DNS.1 = correct.hostname
# IP.1 = 192.168.1.1
```

### Issue 4: "no client certificate available"

**Symptoms:**
```
TLS handshake failed: peer did not return a certificate
```

**Causes:**
- Client certificate not configured
- Wrong certificate path
- Certificate/key mismatch

**Solution:**
```bash
# Verify client has cert configured
# Check paths in client config

# Verify cert and key match
openssl x509 -in client.crt -noout -modulus | md5sum
openssl rsa -in client.key -noout -modulus | md5sum

# Test locally
openssl s_client -connect host:port -cert client.crt -key client.key -CAfile ca.crt
```

### Issue 5: "certificate revoked"

**Symptoms:**
```
SSL: certificate verify failed: certificate revoked
```

**Causes:**
- Certificate was revoked by CA
- CRL/OCSP check failure

**Solution:**
```bash
# Check CRL
openssl crl -in crl.pem -noout -text

# Check if cert serial is in CRL
openssl x509 -in cert.crt -noout -serial
# Compare with CRL entries

# Issue new certificate
```

### Issue 6: "wrong key usage"

**Symptoms:**
```
SSL: certificate verify failed: inappropriate use of certificate
```

**Causes:**
- Certificate missing required key usage
- Using client cert for server or vice versa

**Solution:**
```bash
# Check key usage
openssl x509 -in cert.crt -noout -ext keyUsage,extendedKeyUsage

# Should show for server:
# X509v3 Key Usage: Digital Signature, Key Encipherment
# X509v3 Extended Key Usage: TLS Web Server Authentication

# Regenerate with correct extensions
```

### Issue 7: "protocol version mismatch"

**Symptoms:**
```
SSL: wrong version number
SSL: unsupported protocol
```

**Causes:**
- Client trying TLS 1.2, server requires TLS 1.3
- Outdated TLS library

**Solution:**
```rust
// Ensure compatible TLS versions
// Server config:
let config = ServerConfig::builder()
    .with_safe_defaults()  // Supports TLS 1.2 and 1.3
    // ...

// Or enforce TLS 1.3 only:
let config = ServerConfig::builder()
    .with_protocol_versions(&[&version::TLS13])
    // ...
```

### Issue 8: "cipher suite mismatch"

**Symptoms:**
```
SSL: no shared cipher
```

**Causes:**
- No common cipher suites between client and server
- Server using restrictive cipher list

**Solution:**
```rust
// Check server cipher configuration
// Ensure at least one common cipher:

// Recommended server config:
let config = ServerConfig::builder()
    .with_safe_defaults()  // Includes compatible ciphers
    // ...

// Avoid overly restrictive cipher lists
```

## Kubernetes Debugging

### Check Certificate Secrets

```bash
# List TLS secrets
kubectl get secrets -n default | grep tls

# Inspect secret
kubectl get secret api-gateway-tls -n default -o yaml

# Decode and verify certificate
kubectl get secret api-gateway-tls -n default \
  -o jsonpath='{.data.tls\.crt}' | base64 -d | openssl x509 -noout -dates

# Check if cert-manager is working
kubectl get certificates -n default
kubectl describe certificate api-gateway-cert -n default

# Check cert-manager logs
kubectl logs -n cert-manager -l app=cert-manager
```

### Pod Debugging

```bash
# Check if pod mounted certificates
kubectl exec -it pod-name -n default -- ls -la /etc/tls/

# Verify certificate inside pod
kubectl exec -it pod-name -n default -- \
  openssl x509 -in /etc/tls/tls.crt -noout -dates

# Check pod logs for TLS errors
kubectl logs pod-name -n default | grep -i tls

# Test connection from inside pod
kubectl exec -it pod-name -n default -- \
  curl -v --cert /etc/tls/tls.crt --key /etc/tls/tls.key \
  --cacert /etc/tls/ca.crt https://other-service:443/
```

### Network Policy Debugging

```bash
# Check if network policy blocking
kubectl get networkpolicies -n default

# Test connectivity
kubectl run test-pod --rm -it --image=curlimages/curl --restart=Never -- \
  curl -v https://service:443/

# Check service endpoints
kubectl get endpoints service-name -n default
```

## Service Mesh Debugging

### Linkerd

```bash
# Check mTLS status
linkerd viz check

# View mTLS connections
linkerd viz tap deploy/api-gateway -n default

# Check identity
linkerd identity -n default deploy/api-gateway
```

### Istio

```bash
# Check mTLS status
istioctl analyze

# Verify peer authentication
istioctl authn tls-check pod/api-gateway-xyz.default

# Check certificate
istioctl proxy-config secret deploy/api-gateway -n default

# Verify workload identity
istioctl proxy-config endpoint deploy/api-gateway -n default
```

## Performance Issues

### High Handshake Latency

**Diagnosis:**
```bash
# Measure handshake time
time openssl s_client -connect host:port -CAfile ca.crt </dev/null

# Check server load
kubectl top pods -n default
```

**Solutions:**
- Enable session resumption
- Increase connection pooling
- Use TLS 1.3 (faster handshake)
- Check CPU throttling

### Memory Issues

**Diagnosis:**
```bash
# Check memory usage
kubectl top pods -n default

# Check for connection leaks
netstat -an | grep ESTABLISHED | wc -l
```

**Solutions:**
- Configure connection pool limits
- Set idle timeout
- Check for connection leaks in code

## Logging and Monitoring

### Debug Logging Configuration

```rust
// Cargo.toml
[dependencies]
tracing-subscriber = "0.3"
tracing = "0.1"

// In code:
use tracing_subscriber::{fmt, EnvFilter};

fn setup_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("hyper=debug".parse().unwrap())
                .add_directive("rustls=debug".parse().unwrap())
                .add_directive("myapp=trace".parse().unwrap())
        )
        .init();
}
```

```bash
# Run with debug logging
RUST_LOG=trace,hyper=debug,rustls=debug ./myapp
```

### Prometheus Debug Queries

```promql
# TLS handshake failures by reason
rate(tls_handshake_total{status="failed"}[5m])

# Certificate expiry
tls_certificate_expiry_days

# Active TLS connections
tls_connection_active

# Handshake duration
histogram_quantile(0.99, rate(tls_handshake_duration_seconds_bucket[5m]))
```

## Quick Reference

### Error Code Reference

| Error | Likely Cause | Quick Fix |
|-------|-------------|-----------|
| CERT_EXPIRED | Certificate expired | Renew certificate |
| UNKNOWN_CA | Missing trust chain | Add intermediate to chain |
| HOSTNAME_MISMATCH | SAN mismatch | Regenerate with correct SAN |
| NO_CLIENT_CERT | Client cert not sent | Configure client certificate |
| CERT_REVOKED | Certificate revoked | Issue new certificate |
| WRONG_KEY_USAGE | Invalid key usage | Regenerate with correct usage |
| NO_SHARED_CIPHER | Cipher mismatch | Relax cipher restrictions |
| UNSUPPORTED_PROTOCOL | TLS version mismatch | Enable compatible versions |

### Debug Commands Quick Reference

```bash
# Full chain verification
openssl verify -CAfile ca.crt -untrusted intermediate.crt server.crt

# Test mTLS connection
openssl s_client -connect host:port -CAfile ca.crt -cert client.crt -key client.key

# Check certificate expiry
openssl x509 -in cert.crt -noout -checkend 86400

# View certificate
openssl x509 -in cert.crt -noout -text | head -30

# Extract SAN
openssl x509 -in cert.crt -noout -ext subjectAltName

# Verify cert/key match
diff <(openssl x509 -in cert.crt -noout -modulus) <(openssl rsa -in key.key -noout -modulus)

# Check kubernetes certificates
kubectl get certificates && kubectl describe certificate <name>

# Test from inside pod
kubectl exec -it <pod> -- curl -v --cert <cert> --key <key> --cacert <ca> <url>
```
