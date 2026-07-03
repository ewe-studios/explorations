# src.UIComponents Documentation — Deep Dive

Five distinct projects covering the full spectrum of React/Next.js component distribution: from enterprise npm packages to shadcn registries to DOM sandboxing libraries.

| Project | Files | Type | Key Feature |
|---------|-------|------|-------------|
| [kumo](02-kumo.md) | 644 | npm package (`@cloudflare/kumo`) | 35+ components, 30+ primitives, AI registry |
| [remote-dom](09-remote-dom.md) | 207 | npm monorepo (`@remote-dom/*`) | DOM rendering from sandboxed environments |
| [mapcn](05-mapcn.md) | 124 | shadcn registry | MapLibre map components + 8 blocks |
| [table-forge](07-table-forge.md) | 49 | documentation site | MDX-driven table examples |
| [next-maps](06-next-maps.md) | 36 | standalone app | Mapbox GL showcase |

## Navigation

Foundation:

- **[00-overview](00-overview.md)** — What src.UIComponents is, 5-project breakdown
- **[01-architecture](01-architecture.md)** — Architecture diagrams, dependency graphs for all 5 projects

kumo deep dives:

- **[02-kumo](02-kumo.md)** — Cloudflare's component library: components, primitives, theming
- **[03-kumo-packages](03-kumo-packages.md)** — Monorepo packages: docs, figma, screenshot-worker
- **[04-kumo-ci](04-kumo-ci.md)** — CI/CD: changesets, visual regression, release automation
- **[10-kumo-deep](10-kumo-deep.md)** — Every component, every primitive, source-level detail

map projects:

- **[05-mapcn](05-mapcn.md)** — MapLibre registry and blocks
- **[06-next-maps](06-next-maps.md)** — Mapbox integration

tables:

- **[07-table-forge](07-table-forge.md)** — MDX table showcase

cross-cutting:

- **[08-cross-cutting](08-cross-cutting.md)** — Shared patterns across all projects

remote-dom:

- **[09-remote-dom](09-remote-dom.md)** — DOM rendering from sandboxed environments (iframe, web worker)
