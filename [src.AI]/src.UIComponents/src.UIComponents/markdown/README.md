# src.UIComponents Documentation

Four distinct React/Next.js UI component projects, each solving a different problem domain.

## Projects

| Project | Description | Tech Stack | Key Feature |
|---------|-------------|------------|-------------|
| [kumo](02-kumo.md) | Cloudflare's production component library | Base UI, Tailwind CSS v4, React 19 | 35+ accessible components, AI registry, Figma sync |
| [mapcn](05-mapcn.md) | shadcn-style map component registry | MapLibre GL, Next.js 15, shadcn | 8 pre-built map blocks, zero-config setup |
| [next-maps](06-next-maps.md) | Mapbox map showcase for Next.js | Mapbox GL JS, Next.js, shadcn | Search integration, custom markers, theme toggle |
| [table-forge](07-table-forge.md) | MDX-driven table component showcase | Next.js 15, MDX, Tailwind CSS 4 | 10+ table variations with one-click code copy |

## Navigation

Start here:

- **[00-overview](00-overview.md)** — What src.UIComponents is, why these 4 projects exist together
- **[01-architecture](01-architecture.md)** — Architecture diagrams and dependency graphs for all 4 projects

Then deep dive into each:

- **[02-kumo](02-kumo.md)** — Cloudflare's component library: primitives, components, theming, AI registry
- **[03-kumo-packages](03-kumo-packages.md)** — Kumo's monorepo: docs site, Figma plugin, screenshot worker
- **[04-kumo-ci](04-kumo-ci.md)** — CI/CD pipeline: changesets, visual regression testing, release automation

Map projects:

- **[05-mapcn](05-mapcn.md)** — MapLibre-based map registry with 8 pre-built blocks
- **[06-next-maps](06-next-maps.md)** — Mapbox-based map showcase with search and theming

Table project:

- **[07-table-forge](07-table-forge.md)** — Table component examples with MDX documentation

Cross-cutting:

- **[08-cross-cutting](08-cross-cutting.md)** — Shared patterns: shadcn/ui adoption, Tailwind CSS v4, Next.js 15 App Router
