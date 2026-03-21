# Mutual mTLS Production Implementation with Rust

This directory contains a comprehensive exploration and implementation guide for mutual TLS (mTLS) authentication in production environments using Rust.

## Contents

```
rust-mtls/
├── README.md                 # This file
├── exploration.md            # Comprehensive mTLS exploration
├── rust-revision.md          # Complete Rust implementation guide
└── examples/
    ├── server/               # mTLS server example
    │   ├── Cargo.toml
    │   └── src/main.rs
    ├── client/               # mTLS HTTP client example
    │   ├── Cargo.toml
    │   └── src/main.rs
    └── certgen/              # Certificate generation utility
        ├── Cargo.toml
        └── src/main.rs
```

## Quick Start

### 1. Generate Test Certificates

```bash
cd examples/certgen
cargo run --release
```

This generates test certificates in `./certs/`:
- `ca.crt` / `ca.key` - Certificate Authority
- `server.crt` / `server.key` - Server certificate
- `client.crt` / `client.key` - Client certificate

### 2. Run the mTLS Server

```bash
cd examples/server
cargo run --release
```

Server starts on `https://0.0.0.0:8443`

### 3. Run the mTLS Client

```bash
cd examples/client
cargo run --release
```

The client will:
1. Connect to the server using mTLS
2. Fetch its identity as seen by the server
3. Access protected endpoints
4. Test the echo endpoint

## Documentation

### exploration.md

Comprehensive exploration covering:
- What is mTLS and when to use it
- Production requirements checklist
- PKI hierarchy and certificate lifecycle
- Rust ecosystem and recommended crates
- Architecture patterns
- Security best practices
- Testing strategies
- Deployment considerations
- Migration path

### rust-revision.md

Complete implementation guide with:
- Workspace structure and crate breakdown
- Type system design
- Error handling strategy
- Complete code examples
- Integration test patterns
- Production deployment configurations

## Examples

### Server Example

```rust
use mtls_core::{MtlsConfig, ClientAuthMode};
use mtls_server::MtlsServer;

let config = MtlsConfig {
    server_cert_path: "/etc/certs/server.crt".into(),
    server_key_path: "/etc/certs/server.key".into(),
    ca_cert_path: "/etc/certs/ca.crt".into(),
    client_auth: ClientAuthMode::Required,
    ..Default::default()
};

let server = MtlsServer::builder(config).build().await?;
```

### Client Example

```rust
use mtls_client::{MtlsClient, ClientCertConfig};

let config = ClientCertConfig {
    cert_path: "/etc/certs/client.crt".into(),
    key_path: "/etc/certs/client.key".into(),
    ca_cert_path: "/etc/certs/ca.crt".into(),
};

let client = MtlsClient::builder(config).build()?;
let response = client.get("https://api.example.com/protected").await?;
```

## Production Checklist

- [ ] Replace test CA with production CA
- [ ] Configure certificate rotation
- [ ] Enable monitoring and alerting
- [ ] Set up OCSP/CRL checking
- [ ] Configure proper file permissions (600 for keys)
- [ ] Use HSM or secure enclave for key storage
- [ ] Document runbooks for certificate incidents
- [ ] Test failover and recovery procedures

## Related Explorations

- [Security Exploration](../utm-dev-production/security-signing-exploration.md)
- [Process Compose Rust Revision](../src.process-compose/rust-revision.md)

## License

MIT
