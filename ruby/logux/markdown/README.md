# Logux Deep Exploration — Index

A technical exploration of the Logux client-server synchronization framework: `@logux/core` 0.12.1, `@logux/server` 0.17.1, and `@logux/actions` 0.8.1 (source at `/Users/darkvoid/GitSources/src.Ruby/`).

- [00-what-is-logux.md](./00-what-is-logux.md) — What Logux is: one action log replicated both ways over WebSocket, and the map of the three packages.
- [01-action-and-meta.md](./01-action-and-meta.md) — The action object and its metadata: type, id, time, added, reasons, and the lifetime rules.
- [02-log-and-node.md](./02-log-and-node.md) — The `Log` and `MemoryStore`, the node state machine, and `isFirstOlder` as the deterministic order.
- [03-wire-protocol.md](./03-wire-protocol.md) — The wire protocol: ten message shapes, the handshake, relative time, and text versus binary encoding.
- [04-server-and-subscriptions.md](./04-server-and-subscriptions.md) — The server: `type` and `channel` callbacks, `Context`, subscriptions, and the Publisher.
- [05-offline-connect-reconnect.md](./05-offline-connect-reconnect.md) — Offline-first: watermarks, `Reconnect` backoff, ping keepalive, and resume instead of restart.
- [06-sync-map.md](./06-sync-map.md) — SyncMap: a shared key-value collection with per-field, time-stamped conflict resolution.
- [07-processing-lifecycle.md](./07-processing-lifecycle.md) — The lifecycle: process, `logux/processed`, `logux/undo`, and `logux/prepare`.
- [08-builtin-actions.md](./08-builtin-actions.md) — The remaining built-in actions: subscriptions, define-action, crdt-table, zero-knowledge, shadow, logux-not-found.
- [09-composition.md](./09-composition.md) — How core, server, and actions compose into one machine, and the integration surface that holds them.
