# What Is src.UIComponents?

src.UIComponents is a collection of four distinct React/Next.js UI component projects housed under a single directory. Each project solves a different problem in the UI component space — from enterprise-grade component libraries to specialized map and table components.

## The Four Projects

### kumo — Cloudflare's Production Component Library

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/kumo/`

Kumo (`@cloudflare/kumo` v2.6.0) is Cloudflare's accessible, design-system-compliant UI component library built on [Base UI](https://base-ui.com/) and Tailwind CSS v4. It is a pnpm monorepo with four packages:

- **`@cloudflare/kumo`** — The core library with 35+ components (Button, Dialog, Select, Sidebar, Table, Chart, etc.)
- **`kumo-docs-astro`** — Astro-based documentation site deployed to Cloudflare Pages
- **`kumo-figma`** — Figma plugin for bidirectional design-token synchronization
- **`kumo-screenshot-worker`** — Cloudflare Worker for generating component screenshots

**Aha:** Kumo separates *primitives* (unstyled Base UI wrappers like `primitives/button`, `primitives/dialog`) from *components* (fully styled, opinionated implementations like `components/button`, `components/dialog`). Primitives give you accessibility without opinions; components give you Cloudflare's design system out of the box. Both are exported from the same package via distinct entry points.

```tsx
// Granular import with tree-shaking — only Button code ships
import { Button } from "@cloudflare/kumo/components/button";
// Styles as a separate CSS import
import "@cloudflare/kumo/styles";
```

**Peer dependencies:** `react ^18 || ^19`, `react-dom ^18 || ^19`, `@phosphor-icons/react ^2.1.10`. Optional: `echarts ^6.0.0` (charts), `zod ^4.0.0` (validation).

**Key design constraint:** No `dark:` Tailwind variant allowed — Kumo uses CSS `light-dark()` function for automatic theme handling. This is enforced by a custom ESLint plugin (`lint/no-tailwind-dark-variant.js`).

### mapcn — shadcn-Style Map Components

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/mapcn/`

mapcn is a shadcn/ui-style component registry for maps. Instead of publishing an npm package, it uses the `shadcn` CLI to copy component source directly into your project. The core offering is a single `Map` component powered by [MapLibre GL](https://maplibre.org/) with 8 pre-built block examples:

| Block | Description | Files |
|-------|-------------|-------|
| `map` | Base MapLibre component with markers, popups, routes, arcs, GeoJSON, clusters | `src/registry/map.tsx` |
| `analytics-map` | World map + overview cards + device stats | 4 files |
| `choropleth` | Country shading by metric with hover tooltip | 3 files |
| `uptime-monitor` | Edge network health map with status sidebar | 4 files |
| `logistics-network` | Domestic logistics map with filter sidebar | 4 files |
| `store-locator` | Searchable store list synced to map markers | 4 files |
| `heatmap` | Globe-projected earthquake density heatmap | 1 file |
| `delivery-tracker` | Live order tracking with route progress | 2 files |

```bash
# Install the map component into your project
npx shadcn@latest add "https://mapcn.dev/r/map.json"
```

**Aha:** mapcn's registry JSON (`registry.json`, 345 lines) follows the shadcn schema exactly, meaning any project using `npx shadcn add` can pull mapcn components without installing mapcn as a dependency. Components are copied, not imported — you own the code after installation.

### next-maps — Mapbox Showcase

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/next-maps/`

next-maps is a Next.js showcase project demonstrating Mapbox GL JS integration with shadcn/ui components. Unlike mapcn (which is a reusable component registry), next-maps is a standalone application you can fork.

**Stack:** Next.js (App Router), Tailwind CSS, shadcn/ui, Mapbox GL JS, Mapbox Searchbox API.

**Key components:**
- `src/context/map-context.ts` — React context provider exposing the Mapbox map instance
- `src/lib/mapbox/provider.tsx` — Mapbox lifecycle wrapper (init, cleanup, event delegation)
- `src/components/map/map-search.tsx` — Searchbox integration with debounced results
- `src/components/map/map-controls.tsx` — Zoom, style toggle, geolocation controls

**Aha:** next-maps uses a custom `useDebounce` hook (`src/hooks/useDebounce.ts`) for the search input, showing how to integrate Mapbox's Searchbox API without pulling in the full Searchbox SDK.

### table-forge — MDX Table Showcase

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/table-forge/`

table-forge is a documentation site showcasing 10+ table component variations built with Tailwind CSS. Each table has an MDX documentation page with live preview and one-click code copy.

**Table variations:** BasicBordered, Compact, Filterable, Minimalist, Paginated, Sortable, StripedRows, TopControlsSortable, WithActions, ResponsiveCardsOnMobile.

**Tech:** Next.js 15 (App Router), React 19, Tailwind CSS 4, MDX via `@next/mdx`, shadcn/ui (Button, Card, Tabs).

```bash
# table-forge uses Vercel Analytics and next-themes
# but zero extra UI dependencies for the tables themselves
```

**Aha:** table-forge's tables have **zero dependencies** beyond React + Tailwind. The MDX files (`src/examples/tables/*.mdx`) are processed by `@next/mdx` at build time, making each example a self-contained documentation unit with live code preview.

## Why These Projects Together?

These four projects represent different approaches to component distribution:

| Approach | Project | Distribution Model |
|----------|---------|-------------------|
| npm package | kumo | `pnpm add @cloudflare/kumo` — published to npm, tree-shakable ESM exports |
| shadcn registry | mapcn | `npx shadcn add` — source copied into your project, you own it |
| standalone app | next-maps | Fork and customize — a complete Next.js application |
| documentation site | table-forge | Reference and copy JSX — MDX-generated examples |

Understanding all four gives you the full spectrum of how modern React components get distributed — from fully managed npm packages to copy-paste source code.

## Next

Continue to [01-architecture](01-architecture.md) for architecture diagrams and dependency graphs of each project.
