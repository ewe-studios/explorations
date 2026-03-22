# Taubyte Exploration Index

**Date:** 2026-03-22
**Root Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/`
**Explorations Location:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/taubyte/`

---

## Overview

This index provides a comprehensive overview of all Taubyte component explorations conducted on 2026-03-22. The explorations cover SDKs, core components, and supporting libraries that make up the Taubyte decentralized cloud computing platform.

---

## Exploration Documents

### SDKs

| # | Component | Document | Location |
|---|-----------|----------|----------|
| 1 | **Rust SDK** | [rust-sdk-exploration.md](./rust-sdk-exploration.md) | `rust-sdk/` |
| 2 | **Go SDK** | [go-sdk-exploration.md](./go-sdk-exploration.md) | `go-sdk/` |
| 3 | **AssemblyScript SDK** | [assemblyscript-sdk-exploration.md](./assemblyscript-sdk-exploration.md) | `assemblyscript-sdk/` |
| 4 | **Go SDK Errors** | [go-sdk-supplementary-exploration.md](./go-sdk-supplementary-exploration.md#1-go-sdk-errors) | `go-sdk-errors/` |
| 5 | **Go SDK SmartOps** | [go-sdk-supplementary-exploration.md](./go-sdk-supplementary-exploration.md#2-go-sdk-smartops) | `go-sdk-smartops/` |
| 6 | **Go SDK Symbols** | [go-sdk-supplementary-exploration.md](./go-sdk-supplementary-exploration.md#3-go-sdk-symbols) | `go-sdk-symbols/` |

### Core Components

| # | Component | Document | Location |
|---|-----------|----------|----------|
| 7 | **P2P Library** | [p2p-exploration.md](./p2p-exploration.md) | `p2p/` |
| 8 | **VM (TVM)** | [vm-exploration.md](./vm-exploration.md) | `vm/` |
| 9 | **Wazero Runtime** | [wazero-exploration.md](./wazero-exploration.md) | `wazero/` |
| 10 | **BLS Threshold Cryptography** | [blsttc-exploration.md](./blsttc-exploration.md) | `blsttc/` |
| 11 | **HTTP Utilities** | [http-exploration.md](./http-exploration.md) | `http/` |

---

## Architecture Summary

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Taubyte Platform                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │  Rust SDK   │  │   Go SDK    │  │AssemblyScript│                │
│  │             │  │  SmartOps   │  │    SDK      │                 │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘                 │
│         │                │                │                         │
│         └────────────────┼────────────────┘                         │
│                          │                                          │
│                          ▼                                          │
│              ┌───────────────────────┐                             │
│              │   go-sdk-symbols      │                             │
│              │   (Host Function API) │                             │
│              └───────────┬───────────┘                             │
│                          │                                          │
│                          ▼                                          │
│  ┌───────────────────────────────────────────────────────────┐     │
│  │           Taubyte VM (TVM) - Wazero Runtime               │     │
│  │                                                           │     │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │     │
│  │  │  Backends   │  │   Loader    │  │  Resolver   │       │     │
│  │  │  (DFS/File) │  │             │  │             │       │     │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │     │
│  └───────────────────────────────────────────────────────────┘     │
│                          │                                          │
│                          ▼                                          │
│  ┌───────────────────────────────────────────────────────────┐     │
│  │                    P2P Layer                               │     │
│  │         (libp2p + ipfs-lite + PubSub)                      │     │
│  │                                                            │     │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │     │
│  │  │   libp2p    │  │  ipfs-lite  │  │  GossipSub  │        │     │
│  │  │   (DHT)     │  │  (Storage)  │  │  (PubSub)   │        │     │
│  │  └─────────────┘  └─────────────┘  └─────────────┘        │     │
│  └───────────────────────────────────────────────────────────┘     │
│                          │                                          │
│                          ▼                                          │
│  ┌───────────────────────────────────────────────────────────┐     │
│  │                 Supporting Libraries                       │     │
│  │                                                            │     │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │     │
│  │  │  blsttc     │  │    http     │  │   utils     │        │     │
│  │  │  (BLS TC)   │  │  (Server)   │  │             │        │     │
│  │  └─────────────┘  └─────────────┘  └─────────────┘        │     │
│  └───────────────────────────────────────────────────────────┘     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
Developer Code (Rust/Go/AssemblyScript)
         │
         ▼
┌─────────────────┐
│  SDK (High API) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ SDK Symbols     │──────► Mock (testing)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  TVM (Wazero)   │
│  Host Functions │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  P2P Network    │
│  (libp2p/IPFS)  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Distributed    │
│  Storage/Compute│
└─────────────────┘
```

---

## Key Technologies

### Programming Languages

| Language | Usage | SDKs |
|----------|-------|------|
| Go | Core infrastructure, P2P, VM | go-sdk, go-sdk-* |
| Rust | WASM modules, cryptography | rust-sdk, blsttc |
| TypeScript/AssemblyScript | WASM modules | assemblyscript-sdk |

### Core Technologies

| Technology | Purpose | Components |
|------------|---------|------------|
| WebAssembly | Module execution | vm, all SDKs |
| Wazero | WASM runtime | vm |
| libp2p | P2P networking | p2p |
| IPFS-lite | Distributed storage | p2p |
| GossipSub | Pub/sub messaging | p2p |
| BLS12-381 | Threshold cryptography | blsttc |
| Pebble | Key-value datastore | p2p |

---

## Component Relationships

### SDK Dependencies

```
go-sdk
├── go-sdk-symbols (host function wrappers)
├── go-sdk-errors (error codes)
└── taubyte-sdk (Rust SDK - equivalent)

go-sdk-smartops
├── go-sdk (base SDK)
└── go-sdk-symbols

assemblyscript-sdk
└── taubyte-sdk (equivalent functionality)
```

### VM Dependencies

```
vm (TVM)
├── wazero (runtime)
├── p2p (module loading via DFS)
├── go-sdk-symbols (host functions)
└── http (event handling)
```

### P2P Dependencies

```
p2p
├── libp2p (core networking)
├── ipfs-lite (storage)
├── go-libp2p-pubsub (messaging)
├── go-datastore (storage abstraction)
└── go-ds-pebble (datastore implementation)
```

---

## Summary Statistics

| Category | Count |
|----------|-------|
| SDK Documents | 4 |
| Core Component Documents | 5 |
| Supplementary Documents | 2 |
| **Total Documents** | **11** |
| Total Lines of Documentation | ~5,000+ |
| Components Explored | 11 |
| Source Directories | 11 |

---

## Quick Reference

### Error Codes

See: [go-sdk-supplementary-exploration.md](./go-sdk-supplementary-exploration.md#12-error-code-categories)

### Module Structure Templates

See individual exploration documents for each component's structure.

### API Patterns

| Pattern | Description | Example |
|---------|-------------|---------|
| Resource ID | All VM resources identified by u32 | `Database { id: u32 }` |
| Memory Views | Zero-copy memory sharing | `ReadSeekCloser::open(id)` |
| Host Functions | VM exports via symbols | `taubyte_db_new(...)` |
| Error Handling | errno-style error codes | `Errno::ErrorNone` |

---

## Maintainers

- Sam Stoltenberg (@skelouse)
- Tafseer Khan (@tafseer-khan)
- Samy Fodil (@samyfodil)
- Aron Jalbuena (@arontaubyte)

---

## Documentation References

| Resource | URL |
|----------|-----|
| Official Docs | https://tau.how |
| GoDoc (main) | https://pkg.go.dev/github.com/taubyte |
| Wazero | https://wazero.io |
| libp2p | https://docs.libp2p.io |
| IPFS | https://docs.ipfs.io |
| AssemblyScript | https://www.assemblyscript.org |
| WebAssembly | https://webassembly.org |

---

*This index was generated as part of a comprehensive Taubyte codebase exploration.*
