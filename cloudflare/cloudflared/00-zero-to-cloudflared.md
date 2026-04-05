# Zero to cloudflared: Complete Guide

**Last Updated:** 2026-04-05

---

## Table of Contents

1. [Introduction](#introduction)
2. [What is cloudflared?](#what-is-cloudflared)
3. [Core Architecture](#core-architecture)
4. [Installation](#installation)
5. [Quick Start](#quick-start)
6. [Tunnel Configuration](#tunnel-configuration)
7. [Access Integration](#access-integration)
8. [Production Deployment](#production-deployment)
9. [Advanced Features](#advanced-features)

---

## Introduction

**cloudflared** is Cloudflare's command-line tool and networking daemon that proxies traffic between the Cloudflare network and your origins. It enables:

- **Cloudflare Tunnel** - Secure tunneling without opening firewall ports
- **Cloudflare Access** - Zero-trust access to internal resources
- **Private Networks** - Connect cloud/on-prem resources securely
- **Load Balancing** - Health checks and intelligent routing

---

## What is cloudflared?

### The Problem

Traditional approaches to exposing services:

1. **Port Forwarding** - Opens firewall, security risk
2. **Public IPs** - Exposes origin to attacks
3. **VPNs** - Complex setup, poor UX for users
4. **Bastion Hosts** - Single point of failure

### The cloudflared Solution

```
┌─────────────────────────────────────────────────────────────┐
│                    Cloudflare Global Network                 │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐                │
│  │   Edge    │  │   Edge    │  │   Edge    │                │
│  │  (POP)    │  │  (POP)    │  │  (POP)    │                │
│  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘                │
│        │              │              │                       │
│        └──────────────┼──────────────┘                       │
│                       │                                      │
│                 ┌─────▼─────┐                                │
│                 │ cloudflared│                               │
│                 │  Tunnel    │                               │
│                 └─────┬─────┘                                │
│                       │                                      │
│            ┌──────────┼──────────┐                          │
│            │          │          │                          │
│      ┌─────▼──┐  ┌────▼───┐  ┌──▼────┐                     │
│      │ Web    │  │  SSH   │  │  API  │                     │
│      │ Server │  │ Server │  │ Server│                     │
│      └────────┘  └────────┘  └───────┘                     │
│              Your Private Origin                            │
└─────────────────────────────────────────────────────────────┘
```

### Key Features

| Feature | Description |
|---------|-------------|
| **Zero Trust** | No open ports, origin never directly accessible |
| **QUIC/HTTP3** | Modern transport protocol for efficiency |
| **Automatic TLS** | Certificates managed by Cloudflare |
| **Load Balancing** | Health checks, failover, geographic routing |
| **Access Control** | Integration with Cloudflare Access |
| **Observability** | Metrics, logs, tracing built-in |

---

## Core Architecture

### Components

```
cloudflared/
├── cmd/cloudflared/        # Main CLI entry point
├── connection/             # QUIC/HTTP2 connection handling
├── tunnel/                 # Tunnel management
├── access/                 # Access authentication
├── ingress/                # Traffic routing rules
├── supervisor/             # Process supervision
├── metrics/                # Prometheus metrics
└── logger/                 # Structured logging (zerolog)
```

### Connection Flow

```typescript
// Simplified connection flow

1. Authentication
   cloudflared → Cloudflare CA → JWT Token

2. Tunnel Establishment
   cloudflared → QUIC Edge → Tunnel Routing

3. Traffic Proxy
   Client Request → Cloudflare Edge → cloudflared → Origin
   Origin Response → cloudflared → Cloudflare Edge → Client
```

### Protocol Stack

```
┌──────────────────────────────────────┐
│         Application Layer             │
│  (HTTP, SSH, RDP, TCP, UDP)          │
├──────────────────────────────────────┤
│         QUIC / HTTP3                  │
│  (or HTTP2 for compatibility)        │
├──────────────────────────────────────┤
│         TLS 1.3                       │
│  (mutual authentication)             │
├──────────────────────────────────────┤
│         UDP / TCP                     │
│  (transport layer)                   │
└──────────────────────────────────────┘
```

---

## Installation

### Official Binaries

```bash
# macOS (Homebrew)
brew install cloudflared

# macOS (Direct download)
curl -L --output cloudflared \
  https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-darwin-amd64
chmod +x cloudflared
sudo mv cloudflared /usr/local/bin

# Linux (Debian/Ubuntu)
curl -L --output cloudflared.deb \
  https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared.deb

# Linux (RPM)
curl -L --output cloudflared.rpm \
  https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.rpm
sudo rpm -i cloudflared.rpm

# Linux (Direct)
curl -L --output cloudflared \
  https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64
chmod +x cloudflared
sudo mv cloudflared /usr/local/bin

# Windows
winget install cloudflare.cloudflared
# Or download from releases page
```

### Docker

```bash
# Pull image
docker pull cloudflare/cloudflared:latest

# Run tunnel
docker run cloudflare/cloudflared:latest tunnel run <TUNNEL_ID>
```

### Build from Source

```bash
# Requirements
# - Go >= 1.24
# - GNU Make
# - capnp

# Clone repository
git clone https://github.com/cloudflare/cloudflared.git
cd cloudflared

# Build
make cloudflared

# Test
make test

# Install
sudo make install
```

---

## Quick Start

### TryCloudflare (No Account Required)

```bash
# Expose local server temporarily
cloudflared tunnel --url http://localhost:8080

# Output:
# +----------------------------------------------------+
# |  Your quick Tunnel has been created!               |
# |  Visit it at (it may take some time to be reachable):  |
# |  https://random-name.trycloudflare.com             |
# +----------------------------------------------------+
```

### Named Tunnel (Requires Cloudflare Account)

#### Step 1: Authenticate

```bash
cloudflared tunnel login

# Opens browser for OAuth authentication
# Downloads certificate to ~/.cloudflared/<TUNNEL_ID>.json
```

#### Step 2: Create Tunnel

```bash
cloudflared tunnel create my-tunnel

# Output:
# Created tunnel my-tunnel with id <TUNNEL_ID>
# Credentials saved to ~/.cloudflared/<TUNNEL_ID>.json
```

#### Step 3: Configure Tunnel

```yaml
# ~/.cloudflared/config.yml
tunnel: my-tunnel
credentials-file: /home/user/.cloudflared/<TUNNEL_ID>.json

ingress:
  - hostname: app.example.com
    service: http://localhost:8080
  - hostname: api.example.com
    service: http://localhost:3000
  - service: http_status:404  # Default fallback
```

#### Step 4: Route DNS

```bash
# Route hostname to tunnel
cloudflared tunnel route dns my-tunnel app.example.com
cloudflared tunnel route dns my-tunnel api.example.com

# Or route via load balancer
cloudflared tunnel route lb my-tunnel app.example.com <LB_POOL_ID>
```

#### Step 5: Run Tunnel

```bash
# Run tunnel
cloudflared tunnel run my-tunnel

# Or run with config file
cloudflared tunnel --config ~/.cloudflared/config.yml run
```

---

## Tunnel Configuration

### Ingress Rules

```yaml
# ~/.cloudflared/config.yml

tunnel: my-tunnel
credentials-file: /path/to/credentials.json

# Origin server configuration
originRequest:
  connectTimeout: 30s
  http2Origin: true
  noTLSVerify: false
  caPool: /path/to/ca.pem

# Ingress rules (evaluated in order)
ingress:
  # Route by hostname
  - hostname: app.example.com
    service: http://localhost:8080
    path: /api/*
    originRequest:
      timeout: 60s

  # Route by path
  - hostname: www.example.com
    path: /static/*
    service: http://localhost:3000

  # WebSocket support
  - hostname: chat.example.com
    service: http://localhost:9000
    originRequest:
      http2Origin: true

  # TCP service (SSH)
  - hostname: ssh.example.com
    service: ssh://localhost:22

  # Default (catch-all)
  - service: http_status:404
```

### Origin Services

```yaml
# HTTP origin
- service: http://localhost:8080

# HTTPS origin (with custom CA)
- service: https://localhost:8443
  originRequest:
    caPool: /path/to/ca.pem

# Unix socket
- service: unix:/var/run/app.sock

# SSH
- service: ssh://localhost:22

# RDP
- service: rdp://localhost:3389

# Generic TCP
- service: tcp://localhost:5432
```

### Load Balancing

```yaml
# Multiple origins with health checks
ingress:
  - hostname: app.example.com
    service: load_balancing:
      - origin: http://origin1:8080
        weight: 0.5
        healthCheck:
          path: /health
          interval: 10s
          timeout: 5s
      - origin: http://origin2:8080
        weight: 0.5
        healthCheck:
          path: /health
          interval: 10s
          timeout: 5s
```

---

## Access Integration

### What is Cloudflare Access?

Cloudflare Access provides **Zero Trust authentication** for internal resources:

```
┌─────────────┐     ┌─────────────────┐     ┌─────────────┐
│    User     │ ──► │ Cloudflare Access│ ──► │   Origin    │
│  (Browser)  │     │   (JWT Auth)    │     │  (Protected)│
└─────────────┘     └─────────────────┘     └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  Identity   │
                    │  Provider   │
                    │ (Okta, etc) │
                    └─────────────┘
```

### Configure Access Policy

```bash
# Create Access application
# 1. Go to Zero Trust Dashboard
# 2. Access → Applications → Add Application
# 3. Select "Self-hosted"
# 4. Configure:
#    - Name: My App
#    - Domain: app.example.com
#    - Policies: Allow emails @example.com
```

### Access-Protected Tunnel

```yaml
# ~/.cloudflared/config.yml

tunnel: my-tunnel
credentials-file: /path/to/credentials.json

# Enable Access verification
access:
  service: https://app.example.com
  aud: "<ACCESS_AUDIENCE_TAG>"

ingress:
  - hostname: app.example.com
    service: access://http://localhost:8080
```

### Access CLI

```bash
# Access protected resource
cloudflared access ssh --hostname ssh.example.com

# Start local proxy
cloudflared access tcp --hostname mysql.example.com --url localhost:3306

# Then connect normally
mysql -h localhost -P 3306
```

---

## Production Deployment

### Systemd Service

```ini
# /etc/systemd/system/cloudflared.service

[Unit]
Description=Cloudflare Tunnel
After=network.target

[Service]
Type=simple
User=cloudflared
Group=cloudflared
ExecStart=/usr/local/bin/cloudflared tunnel run my-tunnel
Restart=always
RestartSec=5
Environment=TUNNEL_LOGLEVEL=info

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable cloudflared
sudo systemctl start cloudflared
sudo systemctl status cloudflared
```

### Docker Compose

```yaml
version: '3.8'

services:
  cloudflared:
    image: cloudflare/cloudflared:latest
    command: tunnel run my-tunnel
    volumes:
      - ./config:/etc/cloudflared
      - credentials.json:/.cloudflared/credentials.json
    environment:
      - TUNNEL_LOGLEVEL=info
    restart: unless-stopped
    networks:
      - internal

  app:
    image: my-app:latest
    networks:
      - internal

networks:
  internal:
    internal: true
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cloudflared
spec:
  replicas: 2  # High availability
  selector:
    matchLabels:
      app: cloudflared
  template:
    metadata:
      labels:
        app: cloudflared
    spec:
      containers:
      - name: cloudflared
        image: cloudflare/cloudflared:latest
        command:
        - tunnel
        - run
        - my-tunnel
        volumeMounts:
        - name: credentials
          mountPath: /.cloudflared
          readOnly: true
        - name: config
          mountPath: /etc/cloudflared
          readOnly: true
        env:
        - name: TUNNEL_LOGLEVEL
          value: info
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
      volumes:
      - name: credentials
        secret:
          secretName: cloudflared-credentials
      - name: config
        configMap:
          name: cloudflared-config
```

---

## Advanced Features

### Health Checks

```bash
# View tunnel status
cloudflared tunnel info my-tunnel

# Check origin health
cloudflared tunnel ingress check
```

### Metrics

```yaml
# Enable Prometheus metrics
metrics: 0.0.0.0:2000

# Or with authentication
metrics:
  address: 0.0.0.0:2000
  user: prometheus
  pass: secret
```

### Logging

```bash
# Log levels
cloudflared tunnel run --loglevel debug my-tunnel
cloudflared tunnel run --loglevel info my-tunnel
cloudflared tunnel run --loglevel warn my-tunnel
cloudflared tunnel run --loglevel error my-tunnel

# Log to file
cloudflared tunnel run --logfile /var/log/cloudflared.log my-tunnel

# Log to directory (rotating)
cloudflared tunnel run --logdir /var/log/cloudflared my-tunnel
```

### Multiple Tunnels

```bash
# List all tunnels
cloudflared tunnel list

# Run specific tunnel
cloudflared tunnel run tunnel-1
cloudflared tunnel run tunnel-2

# Or run all configured tunnels
cloudflared tunnel run
```

---

## Related Documents

- [Deep Dive: Tunnel Protocol](./01-tunnel-protocol-deep-dive.md)
- [Deep Dive: Access Authentication](./02-access-auth-deep-dive.md)
- [Deep Dive: Origin Connector](./03-origin-connector-deep-dive.md)
- [Rust Revision](./rust-revision.md)
- [Production Guide](./production-grade.md)
