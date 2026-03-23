---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.lunatic/as-lunatic
repository: https://github.com/lunatic-solutions/as-lunatic
explored_at: 2026-03-23T00:00:00Z
language: AssemblyScript (TypeScript)
---

# Project Exploration: as-lunatic

## Overview

`as-lunatic` is the AssemblyScript SDK for the lunatic runtime. It provides TypeScript-flavored bindings that allow developers to write lunatic applications in AssemblyScript (a TypeScript-to-Wasm compiler), giving them access to processes, message passing, networking, distributed computing, filesystem operations, and the registry -- all the same host APIs available to Rust lunatic applications.

The library is published as an npm package (v0.12.0) and relies on `@ason/assembly` for serialization, `@assemblyscript/wasi-shim` for WASI compatibility, and `as-disposable` for resource cleanup.

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.lunatic/as-lunatic`
- **Remote:** `https://github.com/lunatic-solutions/as-lunatic`
- **Primary Language:** AssemblyScript (TypeScript)
- **License:** MIT

## Directory Structure

```
as-lunatic/
  package.json              # npm package config (v0.12.0)
  asconfig.json             # AssemblyScript compiler config
  tsconfig.json             # TypeScript config
  assembly/
    index.ts                # Main barrel export
    entry.ts                # Entry point utilities
    util.ts                 # Shared utilities
    tests.ts                # Test suite
    globals.d.ts            # Global type declarations
    distributed/
      bindings.ts           # Raw host function imports for distributed ops
      index.ts              # High-level distributed API
    error/
      bindings.ts           # Raw error host function imports
      index.ts              # Error management (Result<T> pattern)
    fs/
      async.ts              # Async filesystem operations
      sync.ts               # Sync filesystem operations
      unsafe.ts             # Unsafe/raw filesystem operations
      util.ts               # FS utilities
    managed/
      held.ts               # Held resource wrapper
      index.ts              # Managed resource exports
      maybe.ts              # Maybe/Option type
      yieldable.ts          # Yieldable process pattern
    message/
      bindings.ts           # Raw message passing host imports
      index.ts              # High-level messaging API
      util.ts               # Message utilities
    net/
      bindings.ts           # Raw networking host imports
      dns.ts                # DNS resolution
      tcp.ts                # TCP server/client (TCPServer, TCPSocket)
      udp.ts                # UDP operations
      util.ts               # Network utilities
    process/
      bindings.ts           # Raw process management host imports
      index.ts              # Process class (spawn, send, receive)
      sandbox.ts            # Process sandboxing/configuration
      util.ts               # Process utilities
    registry/
      bindings.ts           # Raw registry host imports
      index.ts              # Named process registry
    wasi/
      bindings.ts           # WASI bindings
```

## Architecture

### Binding Layers

Each subsystem follows a two-layer pattern:

1. **bindings.ts** - Low-level `@external` function declarations that map directly to lunatic's Wasm host imports (e.g., `lunatic::process::spawn_v1`, `lunatic::networking::tcp_bind_v1`). These are raw i32/i64/f64 interfaces.

2. **index.ts** - High-level TypeScript-idiomatic wrapper classes. For example, `TCPServer` wraps the raw bind/accept host calls, `Process` wraps spawn/send/receive, and `Result<T>` wraps error-checked returns.

### Key Components

- **Process**: Static methods `Process.inheritSpawn<T>()` and `Process.spawnInheritWith<T, U>()` create new processes. Each process gets a `Mailbox<T>` for receiving typed messages.
- **Result<T>**: All fallible operations return `Result<T>` which integrates with `as-disposable` for automatic resource cleanup, preventing lunatic error ID leaks.
- **TCPServer / TCPSocket**: Network primitives following the lunatic networking API. TCP servers bind to addresses and accept connections; sockets read/write buffers.
- **Message Passing**: Uses ASON (AssemblyScript Object Notation) serialization for transferring data between processes.
- **DNS Resolution**: `resolve()` function maps domain names to IP addresses via lunatic host functions.
- **Distributed**: APIs for spawning processes on remote lunatic nodes.
- **Registry**: Named process registration and lookup.
- **Filesystem**: Sync and async file operations through WASI + lunatic extensions.
- **Sandbox**: Process configuration (memory limits, permissions) mirroring `ProcessConfig` in the Rust SDK.

### Serialization

The library depends on `@ason/assembly` (v0.11.1) for message serialization. ASON is a binary serialization format designed for AssemblyScript that handles the memory layout of AS objects. This is combined with the `as-lunatic-transform` compiler plugin which auto-generates `__lunaticSerialize` methods on all classes.

## Dependencies

| Dependency | Version | Purpose |
|-----------|---------|---------|
| @ason/assembly | 0.11.1 | Binary serialization for message passing |
| @assemblyscript/wasi-shim | ^0.1.0 | WASI polyfill layer |
| as-disposable | ^0.1.2 | Resource disposal (error ID cleanup) |
| assemblyscript (dev) | ^0.24.1 | AssemblyScript compiler |

## Ecosystem Role

`as-lunatic` is the second-language SDK for lunatic (after Rust). It demonstrates that the lunatic host API is language-agnostic -- any language that compiles to Wasm can build lunatic applications. The AS SDK provides the same core primitives (processes, mailboxes, networking, distribution) as `lunatic-rs`, making lunatic accessible to the TypeScript/JavaScript developer community.

The project works in tandem with `as-lunatic-transform`, which handles the compile-time code generation needed for serialization.
