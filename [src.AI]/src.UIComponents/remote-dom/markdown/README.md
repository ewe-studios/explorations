# remote-dom Documentation

Render React, Vue, Svelte, or Preact UI from a remote context (worker, iframe, process) into a host app's DOM. The host owns the DOM; the remote sends serialized mutations. From Shopify.

| Attribute | Value |
|-----------|-------|
| Packages | 6 (@remote-dom/core, react, polyfill, signals, compat, elements) |
| Source files | ~235 |
| Framework examples | 6 (React, Preact, Vue, Svelte, Vanilla, HTM) |
| Transport | Direct, iframe, Web Worker |

## Navigation

| # | Document | Purpose |
|---|----------|---------|
| 00 | [Overview](00-overview.md) | What it is, architecture, packages |
| 01 | [Core Package](01-core-package.md) — RemoteConnection, receivers, types, elements |
| 02 | [React Package](02-react-package.md) — createRemoteComponent, slot props, event props |
| 03 | [Polyfill Package](03-polyfill-package.md) — DOM API emulation |
| 04 | [Signals Package](04-signals-package.md) — Preact signals receiver |
| 05 | [Compat Package](05-compat-package.md) — Legacy adapter |
| 06 | [Examples](06-examples.md) — Kitchen-sink with 6 frameworks |
| 07 | [Architecture](07-architecture.md) — Full host/remote communication flow |
