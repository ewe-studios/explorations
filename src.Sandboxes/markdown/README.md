# Sandboxes Documentation

Collection of sandbox implementations for secure code execution.

## Document Index

| # | Document | Description |
|---|----------|-------------|
| 00 | [Overview](00-overview.html) | Sandbox landscape |
| 01 | [agent-safehouse](01-agent-safehouse.html) | macOS Seatbelt |
| 02 | [CubeSandbox](02-cubesandbox.html) | KVM microVMs |
| 03 | [deer-flow](03-deer-flow.html) | Super agent |
| 04 | [flue](04-flue.html) | Container framework |
| 05 | [Kami](05-kami.html) | Browser-based |
| 06 | [shuru](06-shuru.html) | MicroVM sandbox |
| 07 | [superhq](07-superhq.html) | Orchestration |
| 08 | [Others](08-others.html) | ml-intern, superpowers |

## Quick Links

- **Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/`

## The Sandbox Spectrum

| Approach | Isolation | Performance | Use Case |
|----------|-----------|-------------|----------|
| Browser extension | Low | High | Client-side AI |
| Container | Medium | High | Development |
| Seatbelt | Medium-High | High | macOS agents |
| MicroVM | High | Medium | Production |
| KVM | Very High | Medium | Security-critical |

## Projects Overview

| Project | Language | Approach |
|---------|----------|----------|
| agent-safehouse | Bash | macOS Seatbelt |
| CubeSandbox | Rust/Go | KVM microVM |
| deer-flow | Python/TS | Container |
| flue | TypeScript | Container |
| Kami | Python | Browser |
| ml-intern | Python | Container |
| shuru | Rust | MicroVM |
| superhq | Rust | Sandboxed orchestration |
| superpowers | TypeScript | Browser extension |
