---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/
language: TypeScript/JavaScript
version: kumo@2.6.0 (latest published)
explored_at: 2026-07-03
license: MIT (all 4 sub-projects)
---

# spec.md — src.UIComponents Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/`
- **Language:** TypeScript/JavaScript (React 18/19, Next.js 15)
- **Projects:** 4 sub-projects (kumo, mapcn, next-maps, table-forge)
- **License:** MIT across all sub-projects
- **Package Managers:** pnpm (kumo workspace), npm (mapcn, next-maps, table-forge)

## What src.UIComponents Is

A collection of four distinct React/Next.js UI component projects, each solving a different problem domain:

1. **kumo** — Cloudflare's production component library (`@cloudflare/kumo` v2.6.0). A pnpm monorepo with 4 packages: the core `@cloudflare/kumo` component library (35+ components built on Base UI), an Astro-based docs site, a Figma plugin for design-token sync, and a screenshot worker. Uses pnpm catalog protocol for dependency management, changesets for versioning, and vitest for testing (unit + browser).

2. **mapcn** — A shadcn/ui-style map component registry built on MapLibre GL. Provides a single `Map` component with markers, popups, routes, arcs, GeoJSON layers, and clusters, plus 8 pre-built block examples (choropleth, analytics dashboard, uptime monitor, logistics network, store locator, heatmap, delivery tracker). Uses the `shadcn` CLI for component installation.

3. **next-maps** — A Next.js-based map showcase built on Mapbox GL JS with shadcn/ui. Features a clean map interface with search (Mapbox Searchbox API), styled markers with custom icons, popups, geolocation, theme toggle, and map style switching.

4. **table-forge** — A table component documentation site showcasing 10+ table variations (basic, sorting, filtering, paginated, striped, compact, with actions, responsive mobile cards). Built with Next.js 15, Tailwind CSS 4, MDX for documentation, and shadcn/ui components.

## Documentation Goal

A reader should understand:

1. What each of the 4 projects does and when to use each
2. kumo's component architecture: primitives vs components, base UI delegation, theming system, AI registry
3. mapcn's registry pattern: how shadcn-style component installation works, the Map component API surface
4. next-maps' Mapbox integration: context providers, map lifecycle, search integration
5. table-forge's example-driven documentation: MDX-based component showcase pattern
6. The CI/CD infrastructure for kumo (changesets, visual regression, release automation)
7. How all four projects relate to each other and the broader ecosystem

## Documentation Structure

```
src.UIComponents/
├── spec.md
├── markdown/
│   ├── README.md                  ← Index / table of contents
│   ├── 00-overview.md             ← What src.UIComponents is, 5-project breakdown
│   ├── 01-architecture.md         ← Architecture of each project, dependency graphs
│   ├── 02-kumo.md                 ← Kumo: Cloudflare's component library deep dive
│   ├── 03-kumo-packages.md        ← Kumo: monorepo packages (docs, figma, screenshot-worker)
│   ├── 04-kumo-ci.md              ← Kumo: CI/CD, changesets, visual regression
│   ├── 05-mapcn.md                ← mapcn: MapLibre registry and blocks
│   ├── 06-next-maps.md            ← next-maps: Mapbox integration
│   ├── 07-table-forge.md          ← table-forge: MDX-driven table showcase
│   ├── 08-cross-cutting.md        ← Patterns across all 5 projects
│   ├── 09-remote-dom.md           ← remote-dom: DOM rendering from sandboxed environments
│   └── 10-kumo-deep.md            ← Kumo: every component, every primitive, source-level
├── html/                          ← Generated HTML
│   ├── index.html
│   ├── styles.css
│   └── *.html
└── build.py                       ← Copied from shared build script
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | README.md (index) | DONE |
| 2 | 00-overview.md | DONE |
| 3 | 01-architecture.md | DONE |
| 4 | 02-kumo.md | DONE |
| 5 | 03-kumo-packages.md | DONE |
| 6 | 04-kumo-ci.md | DONE |
| 7 | 05-mapcn.md | DONE |
| 8 | 06-next-maps.md | DONE |
| 9 | 07-table-forge.md | DONE |
| 10 | 08-cross-cutting.md | DONE |
| 11 | 09-remote-dom.md | DONE |
| 12 | 10-kumo-deep.md | DONE |
| 13 | HTML generation | DONE |
| 14 | Grandfather review | DONE |

## Build System

```bash
cd "[src.AI]/src.UIComponents/src.UIComponents"
python3 build.py src.UIComponents
```

Shared build.py from `/home/darkvoid/Boxxed/@dev/repo-expolorations/build.py` — Python 3.12+ stdlib only, no external dependencies.

## Quality Requirements

All 10 Iron Rules from the documentation directive apply:
1. Detailed sections with code snippets
2. Teach key facts quickly
3. Clear articulation
4. Mermaid diagrams (minimum 2 per document)
5. Good visual assets (tables, code blocks)
6. Generated HTML with navigation
7. Cross-references between documents
8. Source path references
9. Aha moments
10. Prev/next navigation + index

## Expected Outcome

A developer reading these docs can:
- Understand which of the 4 projects fits their use case
- Install and use kumo components with proper import patterns
- Use mapcn's registry to add map components to their project
- Extend next-maps with their own Mapbox features
- Build table-forge-style documentation for their own components

## Resume Point

If interrupted, check `spec.md` tasks table for DONE status. Each document builds on the previous; resume from the first TODO task.
