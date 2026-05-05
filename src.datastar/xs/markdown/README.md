# xs (cross.stream) Documentation

Local-first event streaming store with content-addressable storage, reactive processors, and Nushell integration.

## Documents

1. [Overview](00-overview.md) -- What xs is, core concepts, and system boundaries
2. [Architecture](01-architecture.md) -- Module layout, dependency graph, layer decomposition
3. [Storage Engine](02-storage-engine.md) -- fjall LSM-tree config, cacache CAS, on-disk layout
4. [Frame Model](03-frame-model.md) -- The Frame struct, TTL policies, ReadOptions, write/read paths
5. [SCRU128 IDs](04-scru128-ids.md) -- Time-ordered ID system, components, generation, packing
6. [Indexing Strategy](05-indexing.md) -- Hierarchical topic index, prefix queries, ADR 0001/0002
7. [API & Transport](06-api-transport.md) -- HTTP routes, Unix/TCP/TLS/Iroh transports, SSE/NDJSON
8. [Processor System](07-processor-system.md) -- Actors, Services, Actions, lifecycle, ReturnOptions
9. [Nushell Integration](08-nushell-integration.md) -- Engine, custom commands, VFS modules, xs.nu
10. [CLI Commands](09-cli-commands.md) -- All subcommands, flags, output formats
11. [Production Patterns](10-production-patterns.md) -- Deployment, scaling, export/import, monitoring
