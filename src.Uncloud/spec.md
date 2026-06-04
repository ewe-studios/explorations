---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.cloud_providers/src.Uncloud/uncloud/
repository: github.com/psviderski/uncloud
explored_at: 2026-06-04
documentation_goal: Document Uncloud — lightweight clustering and container orchestration tool that deploys across servers without Swarm or Kubernetes.
---

# Spec: Uncloud Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `src.Uncloud/uncloud/` |
| Language | Go (1.26) |
| License | Apache-2.0 |
| Total LOC | 60,523 |
| Source files | 289 |

| Component | LOC | Purpose |
|-----------|-----|---------|
| `internal/machine` | 21,581 | Core machine/cluster management |
| `pkg/client` | 15,442 | Client library |
| `cmd/uncloud` | 6,165 | CLI commands |
| `pkg/api` | 3,794 | API types and validation |
| `internal/cli` | 2,668 | CLI implementation |
| `internal/machine/api` | 9,696 | gRPC/protobuf API |
| `internal/machine/caddyconfig` | 2,648 | Caddy reverse proxy config |
| `internal/machine/docker` | 2,998 | Docker controller |
| `internal/machine/network` | 1,105 | WireGuard mesh networking |
| `test/e2e` | 4,524 | End-to-end tests |

## 2. What Uncloud Is

Uncloud is a lightweight clustering and container orchestration tool — deploy and scale containerised apps across servers without Swarm or Kubernetes overhead. It creates a secure WireGuard mesh network between Docker hosts and provides automatic service discovery, load balancing, ingress with HTTPS, and simple CLI commands.

Key differentiators:
- **No central control plane** — fully decentralized, P2P state sync via Corrosion (SQLite-based CRDT)
- **WireGuard mesh** — automatic peer discovery and NAT traversal
- **Docker Compose** — familiar compose format, no bespoke DSL
- **Unregistry integration** — build/push directly to machines without external registry
- **Automatic HTTPS** — built-in Caddy reverse proxy with Let's Encrypt

## 3. Documentation Goal

A reader should understand:
1. The decentralized architecture — no control plane, P2P sync
2. WireGuard mesh networking — peer discovery, NAT traversal
3. The machine model — cluster state, Docker integration
4. Service deployment — compose parsing, scheduling, container creation
5. Caddy integration — automatic HTTPS, load balancing
6. The CLI — deploy, ps, service, machine, volume, image commands
7. Corrosion — SQLite-based CRDT for distributed state
8. The API — gRPC services, protobuf definitions

## 4. Documentation Structure

```
src.Uncloud/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-wireguard-mesh.md
│   ├── 03-machine-cluster.md
│   ├── 04-service-deployment.md
│   ├── 05-caddy-https.md
│   ├── 06-cli.md
│   ├── 07-api-grpc.md
│   ├── 08-corrosion-crdt.md
│   ├── 09-docker-integration.md
│   ├── 10-client-library.md
│   ├── 11-cross-cutting.md
├── html/
└── build.py
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-architecture.md | TODO |
| 3 | 02-wireguard-mesh.md | TODO |
| 4 | 03-machine-cluster.md | TODO |
| 5 | 04-service-deployment.md | TODO |
| 6 | 05-caddy-https.md | TODO |
| 7 | 06-cli.md | TODO |
| 8 | 07-api-grpc.md | TODO |
| 9 | 08-corrosion-crdt.md | TODO |
| 10 | 09-docker-integration.md | TODO |
| 11 | 10-client-library.md | TODO |
| 12 | 11-cross-cutting.md | TODO |
| 13 | Grandfather review | TODO |
| 14 | Fix findings | TODO |
| 15 | Generate HTML | TODO |

## 6-9. Standard sections

Build via `python3 build.py .`. Grandfather review mandatory.
