---
title: Caddy & HTTPS — Automatic HTTPS, Load Balancing
---

# Caddy & HTTPS — Automatic HTTPS, Load Balancing

**Uncloud integrates Caddy as the built-in reverse proxy — providing automatic HTTPS with Let's Encrypt, load balancing across service replicas, and zero-config TLS provisioning.**

## Caddy Architecture

Source: `internal/machine/caddyconfig/` (2,648 LOC)

```mermaid
flowchart TB
    subgraph Internet["Internet"]
        I1["HTTPS request"]
    end

    subgraph Caddy["Caddy (on each machine)"]
        C1["TLS termination"]
        C2["Load balancer"]
        C3["Caddyfile config"]
    end

    subgraph Services["Backend Services"]
        S1["web:1 (container A)"]
        S2["web:2 (container B)"]
        S3["web:3 (container C)"]
    end

    I1 --> C1
    C1 --> C2
    C2 --> S1
    C2 --> S2
    C2 --> S3
    C3 -.->|"config"| C1
```

## Caddy Configuration

Source: `internal/machine/caddyconfig/caddyfile.go`

The Caddy config controller generates Caddyfile from service specs:

| Service Spec | Caddy Directive |
|-------------|----------------|
| `CaddySpec.Domain` | `domain.example.com` site block |
| `CaddySpec.TLS` | `tls internal` or `tls email@` |
| `Service.Ports` | `reverse_proxy` upstream list |
| `Service.Replicas` | Multiple upstream entries |

## Caddy Service

Source: `internal/machine/caddyconfig/service.go`

The Caddy service runs as a system service managed by the cluster controller:

| Responsibility | Implementation |
|---------------|----------------|
| Config generation | `caddyfile.go` |
| Config validation | Admin socket API |
| Config reload | Graceful reload (zero downtime) |
| TLS provisioning | Let's Encrypt (automatic) |

## TLS Certificate Management

```mermaid
sequenceDiagram
    participant Client as HTTPS Client
    participant Caddy as Caddy
    participant LE as Let's Encrypt

    Client->>Caddy: HTTPS request (first time)
    Caddy->>LE: Request certificate (ACME)
    LE-->>Caddy: Certificate issued
    Caddy->>Client: Serve HTTPS
    
    Note over Caddy,LE: Certificate auto-renewed before expiry
```

## Load Balancing

When a service has multiple replicas, Caddy distributes requests:

```
reverse_proxy web-abc123:8080 web-def456:8080 web-ghi789:8080
```

Each container's WireGuard IP is used as the upstream — direct encrypted communication, no overlay network needed.

**Aha:** Every machine runs its own Caddy instance. If a machine goes down, services on other machines continue serving traffic. There's no single Caddy bottleneck — the load is distributed across the cluster.

## What's Next

- [06 — CLI](06-cli.md) — Commands, config, connection types
- [04 — Service Deployment](04-service-deployment.md) — Return to deployment
- [09 — Docker Integration](09-docker-integration.md) — Return to Docker
