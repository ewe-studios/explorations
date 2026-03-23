---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.matrix-web/tailscale-k8s
repository: https://github.com/element-hq/tailscale-k8s
explored_at: 2026-03-23
language: Shell
---

# Sub-Project Exploration: tailscale-k8s

## Overview

A minimal Docker container that runs Tailscale in a Kubernetes pod, enabling Tailscale VPN connectivity for Kubernetes workloads. Used by the Element infrastructure team to provide secure network access to internal services from Kubernetes clusters.

## Structure

```
tailscale-k8s/
├── Dockerfile              # Based on ghcr.io/tailscale/tailscale:v1.80.3
├── run.sh                  # Entrypoint script
└── README.md
```

## Key Insights

- Extremely minimal: just a Dockerfile and shell script wrapping the official Tailscale container
- Pins to Tailscale v1.80.3
- Infrastructure utility, not Matrix-specific
- Enables Tailscale mesh VPN within Kubernetes pods
