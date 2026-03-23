# Caddy Web Server - Comprehensive Exploration

## Overview

Caddy is an extensible server platform written in Go that uses TLS by default. It is most known for being the first web server to implement automatic HTTPS by default, securing trillions of connections and managing millions of TLS certificates in production environments.

**Project Structure:**
```
src.caddy/
├── caddy/          # Core Caddy server platform
│   ├── caddyconfig/    # Configuration parsing (Caddyfile, JSON)
│   ├── caddytest/      # Integration tests
│   ├── cmd/caddy/      # Main entry point
│   ├── internal/       # Internal packages
│   ├── modules/        # Modular extensions
│   │   ├── caddyhttp/      # HTTP server module
│   │   ├── caddytls/       # TLS management module
│   │   ├── caddypki/       # PKI/Certificate Authority module
│   │   ├── caddyfs/        # File system module
│   │   ├── logging/        # Logging module
│   │   └── standard/       # Standard modules
│   └── notify/         # System notifications
└── certmagic/      # TLS/ACME automation library (used by Caddy)
```

## Key Features

### 1. Automatic HTTPS
- Obtains certificates from Let's Encrypt, ZeroSSL, or any ACME-compliant CA
- Automatically renews certificates before expiration
- Staples OCSP responses for improved privacy and security
- Supports HTTP->HTTPS redirects out of the box

### 2. Modular Architecture
Caddy uses a powerful module system that allows extending functionality without bloating the core. Modules can hook into:
- HTTP request handling (middleware chain)
- TLS handshakes
- Certificate management
- Storage backends
- Logging systems

### 3. Multiple Configuration Formats
- **Caddyfile**: Human-friendly configuration syntax
- **JSON**: Native configuration format with full API access
- **Config Adapters**: Convert other formats (YAML, TOML, NGINX config) to JSON

### 4. On-Demand TLS
- Obtain certificates during TLS handshakes
- Useful for serving certificates for domains not known ahead of time
- Includes built-in protections against abuse

### 5. Cluster Support
- Coordinate certificate management across multiple instances
- Shared storage backends enable distributed solving of ACME challenges
- Efficient locking prevents duplicate operations

## Core Components

### Caddy Core (`caddy/`)

The core Caddy server provides:
- **Module System**: Dynamic loading and management of extensions
- **Listener Management**: Reusable sockets for graceful reloads
- **Configuration System**: JSON-based config with live reload via admin API
- **Context System**: Propagates configuration and state through the application

Key files:
- `caddy.go`: Main entry point and lifecycle management
- `modules.go`: Module registration and loading system
- `listeners.go`: Network listener management with SO_REUSEPORT support
- `context.go`: Configuration context for modules

### CertMagic (`certmagic/`)

CertMagic is the TLS automation library that powers Caddy's HTTPS features. It can be used standalone in any Go application.

**Capabilities:**
- Automated certificate issuance via ACME protocol
- Automatic renewal with configurable windows
- OCSP stapling and response caching
- Multiple challenge types (HTTP-01, TLS-ALPN-01, DNS-01)
- Distributed challenge solving

Key files:
- `certmagic.go`: High-level convenience functions
- `config.go`: Configuration structure and management
- `acmeissuer.go`: ACME protocol implementation
- `handshake.go`: TLS handshake handling
- `maintain.go`: Background certificate maintenance
- `cache.go`: In-memory certificate cache

### TLS Module (`modules/caddytls/`)

The TLS module integrates CertMagic with Caddy:
- Certificate loading and management
- Automation policies for different domains
- Internal PKI (private CA) support
- Session ticket management
- Encrypted ClientHello (ECH) support

### HTTP Module (`modules/caddyhttp/`)

The HTTP module provides:
- Request routing and handling
- Middleware chain execution
- Static file serving
- Reverse proxy
- Authentication and authorization
- Rate limiting

## Architecture Details

### Module Lifecycle

1. **Registration**: Modules register themselves via `init()` functions
2. **Loading**: Configuration triggers module loading via `Context.LoadModule()`
3. **Provisioning**: `Provisioner.Provision()` called for setup
4. **Validation**: `Validator.Validate()` called to verify configuration
5. **Cleanup**: `CleanerUpper.Cleanup()` called when context is cancelled

### Certificate Management Flow

```
┌─────────────────────────────────────────────────────────────┐
│                   ManageSync/ManageAsync                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│         CacheManagedCertificate / ObtainCert                 │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
              ▼                               ▼
┌─────────────────────────┐       ┌─────────────────────────┐
│  Load from Storage      │       │  Obtain from Issuer     │
│  (if exists)            │       │  (ACME / Internal)      │
└─────────────────────────┘       └─────────────────────────┘
              │                               │
              └───────────────┬───────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Cache in Memory (certCache)                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│           TLS Handshake (GetCertificate)                     │
│           Background Maintenance (Renew/OCSP)                │
└─────────────────────────────────────────────────────────────┘
```

### Challenge Solving Architecture

**HTTP-01 Challenge:**
1. ACME server requests HTTP resource at `http://domain/.well-known/acme-challenge/TOKEN`
2. Caddy registers challenge handler via `HTTPChallengeHandler`
3. Response served with token and key authorization
4. ACME server validates and issues certificate

**TLS-ALPN-01 Challenge:**
1. ACME server connects via TLS with special ALPN protocol `acme-tls/1`
2. Caddy serves special certificate via `GetCertificate` callback
3. Certificate contains validation token in SAN
4. ACME server validates and issues certificate

**DNS-01 Challenge:**
1. ACME server requests DNS TXT record at `_acme-challenge.domain`
2. CertMagic uses libdns provider to create record
3. ACME server validates DNS propagation
4. Record cleaned up, certificate issued

### Storage System

CertMagic uses a pluggable storage system for:
- Certificate and private key persistence
- OCSP response caching
- ACME account information
- Challenge coordination (distributed solving)
- Lock synchronization

Default storage: Local filesystem at `$HOME/.local/share/certmagic` (XDG compliant)

Storage interface methods:
- `Store/load` - Key-value operations
- `List` - Enumerate keys by prefix
- `Delete` - Remove keys
- `Stat` - Get metadata
- `Lock/Unlock` - Distributed locking

## Performance Characteristics

### Memory Usage
- Certificates cached in memory for fast handshake access
- Default cache capacity: 10,000 certificates
- LRU eviction when capacity reached

### Concurrency
- Read-heavy certificate cache with RWMutex
- Fine-grained locking for certificate operations
- Lock queues prevent stampeding on same domain

### Network
- Connection pooling for ACME client
- Configurable timeouts (30s default for TLS handshake)
- HTTP/2 support for ACME communications

## Security Features

1. **Private Key Rotation**: New key for each certificate by default
2. **OCSP Stapling**: Improved privacy and revocation checking
3. **Certificate Transparency**: All issued certs logged to CT logs
4. **Must-Staple Support**: Optional OCSP Must-Staple extension
5. **Internal Rate Limiting**: Protects CA endpoints from firehosing

## ACME Protocol Support

CertMagic uses [ACMEz](https://github.com/mholt/acmez/v3) as the underlying ACME client implementation, supporting:
- RFC 8555 (ACME v2)
- RFC 8737 (TLS-ALPN-01)
- RFC 8738 (IP Address Certificates)
- RFC 9773 (ACME Renewal Information - ARI)
- Encrypted ClientHello (ECH)

## Comparison with Other Servers

| Feature | Caddy | NGINX | Apache |
|---------|-------|-------|--------|
| Automatic HTTPS | Built-in | Via certbot | Via mod_md |
| Configuration | Caddyfile/JSON | Custom DSL | Custom DSL |
| Architecture | Modular (Go) | Modular (C) | Modular (C) |
| Hot Reload | Graceful (socket sharing) | Graceful | Graceful |
| HTTP/3 | Built-in | Module | Module |
| Memory Safety | Go (safe) | C (unsafe) | C (unsafe) |

## References

- [Caddy Documentation](https://caddyserver.com/docs/)
- [CertMagic Documentation](https://pkg.go.dev/github.com/caddyserver/certmagic)
- [ACME Protocol (RFC 8555)](https://datatracker.ietf.org/doc/html/rfc8555)
- [Let's Encrypt Rate Limits](https://letsencrypt.org/docs/rate-limits/)
