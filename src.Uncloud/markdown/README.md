---
title: Uncloud Documentation
---

# Uncloud Documentation

Lightweight clustering and container orchestration — deploy across servers without Swarm or Kubernetes overhead.

## Documents

### Foundation

- [**00 — Overview**](00-overview.md) — What Uncloud is, architecture, key differentiators
- [**01 — Architecture**](01-architecture.md) — Decentralized cluster, P2P design, client-server model

### Core Systems

- [**02 — WireGuard Mesh**](02-wireguard-mesh.md) — Network setup, peer discovery, NAT traversal
- [**03 — Machine & Cluster**](03-machine-cluster.md) — clusterController, state management, Corrosion
- [**04 — Service Deployment**](04-service-deployment.md) — ServiceSpec, scheduling, container lifecycle
- [**05 — Caddy & HTTPS**](05-caddy-https.md) — Automatic HTTPS, load balancing
- [**08 — Corrosion CRDT**](08-corrosion-crdt.md) — P2P state synchronization

### Interface

- [**06 — CLI**](06-cli.md) — Commands, configuration, connection types
- [**07 — API & gRPC**](07-api-grpc.md) — Protobuf definitions, gRPC services
- [**10 — Client Library**](10-client-library.md) — pkg/client API

### Infrastructure

- [**09 — Docker Integration**](09-docker-integration.md) — Docker controller, Unregistry
- [**11 — Cross-Cutting**](11-cross-cutting.md) — Testing, metrics, SSH exec, ucind
