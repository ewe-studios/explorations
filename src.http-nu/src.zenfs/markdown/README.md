# zenfs — Virtual Filesystem for TypeScript

Cross-platform Node.js `fs` API emulation in browsers and other environments. `@zenfs/core` v2.5.6.

## Source

- **Package:** `@zenfs/core` v2.5.6
- **Language:** TypeScript
- **178 source files across 12 packages**
- **License:** LGPL-3.0-or-later

## Documentation

| Document | Description |
|----------|-------------|
| [00-overview](markdown/00-overview.md) | Package structure, architecture, dependencies |
| [01-vfs-architecture](markdown/01-vfs-architecture.md) | VFS layers, Inode format, Store abstraction, Node.js API |
| [02-backends-stores](markdown/02-backends-stores.md) | Six backends: InMemory, Fetch, Port, SingleBuffer, CopyOnWrite, Passthrough |

## Key Features

- **Node.js fs compatible** — sync, async, promises, streams, Dir
- **Pluggable backends** — mount different storage at different paths
- **Transaction support** — atomic operations with rollback
- **Cross-environment** — browsers, Web Workers, Deno, Node.js
- **Chroot-like isolation** — `bindContext()` for sandboxed access
