# Security Model

Moltis is secure by design: keys never leave your machine, every command runs in a sandboxed container, and access is controlled through multiple authentication layers.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/`

## Security Principles

1. **Keys never leave your machine** — API keys, passkeys, and secrets stay local
2. **Sandboxed execution** — Every tool runs in Docker, Apple Container, or WASM
3. **Multi-layer auth** — Password + Passkey + API keys + Vault
4. **Auditable codebase** — ~270K Rust lines, 470+ test files, unsafe code isolated to FFI/precompiled boundaries

## Authentication

```
crates/auth/                  ← Authentication (password, passkey, API keys)
crates/vault/                 ← Secret vault
crates/secret-store/          ← Secret storage
crates/oauth/                 ← OAuth flows
```

**Source:** `crates/auth/src/`, `crates/vault/src/`, `crates/secret-store/src/`, `crates/oauth/src/`.

## Sandboxing

```
crates/tools/                 ← Tool execution with sandboxing
```

Sandbox backends:
- **Docker/Podman** — Container isolation (Linux, macOS)
- **Apple Container** — Native macOS sandboxing
- **WASM** — WebAssembly sandboxing (cross-platform)

## TLS

```
crates/tls/                   ← TLS configuration
```

TLS handling for secure connections.

**Source:** `crates/tls/src/`.

## Network Filter

```
crates/network-filter/        ← Network access control
```

Network filtering controls outbound access from tools and plugins.

**Source:** `crates/network-filter/src/`.

## Node Host

```
crates/node-host/             ← Node.js runtime hosting
```

Node.js hosting for tools that require it, with sandboxing.

**Source:** `crates/node-host/src/`.

## Plugins

```
crates/plugins/               ← Plugin system
```

Plugin system with sandboxed execution and permission controls.

**Source:** `crates/plugins/src/`.

## External Agents

```
crates/external-agents/       ← External agent connections
```

Connections to external agents with security controls.

**Source:** `crates/external-agents/src/`.

## Portable

```
crates/portable/              ← Portable deployment
```

Portable deployment for air-gapped or restricted environments.

**Source:** `crates/portable/src/`.

## QMD

```
crates/qmd/                   ← Query Markdown Documents
```

Query markdown documents tool.

**Source:** `crates/qmd/src/`.

## Onboarding

```
crates/onboarding/            ← Secure onboarding flow
```

Secure first-time setup with guided configuration.

**Source:** `crates/onboarding/src/`.

## CLI

```
crates/ctl/                   ← Control interface (CLI)
crates/cli/                   ← Command-line interface
```

CLI tools for administration and control.

**Source:** `crates/ctl/src/`, `crates/cli/src/`.

## Service Traits

```
crates/service-traits/        ← Service interface definitions
```

Common service interface definitions.

**Source:** `crates/service-traits/src/`.

## Common

```
crates/common/                ← Shared types and utilities
```

Shared types used across the workspace.

**Source:** `crates/common/src/`.

## Config

```
crates/config/                ← Configuration management
```

Configuration loading, validation, and management.

**Source:** `crates/config/src/`.

## Aha Moments

1. **Unsafe code is isolated** — Unsafe Rust code only exists at FFI and precompiled runtime boundaries, not in the core agent loop. This makes the agent loop itself safe to audit.

2. **No plugin marketplace** — Unlike other agent platforms, Moltis doesn't have a plugin marketplace that could be supply-chain attacked. Tools and skills are bundled or explicitly imported.

3. **One binary, no runtime** — The entire server is a single Rust binary with no external runtime dependencies. This eliminates a huge class of supply-chain and deployment security issues.

## Next

You've completed the src.Moltis documentation. Return to the [README](README.md) for the full index.
