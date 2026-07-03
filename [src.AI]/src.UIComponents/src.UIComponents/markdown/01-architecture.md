# Architecture

This document maps the architecture of each of the four src.UIComponents projects, including dependency graphs, layer diagrams, and module relationships.

## kumo Architecture

### Monorepo Structure

```mermaid
flowchart TD
    subgraph Workspace ["pnpm workspace (root)"]
        direction TB
        ROOT["package.json (workspace root)
        devDeps: @changesets/cli, lefthook, oxlint"]
    end

    subgraph Packages ["packages/"]
        KUMO["@cloudflare/kumo
        Core component library
        35+ components, v2.6.0"]
        DOCS["kumo-docs-astro
        Astro docs site
        Deployed to Cloudflare Pages"]
        FIGMA["kumo-figma
        Figma plugin
        Design-token sync"]
        SCREENSHOT["kumo-screenshot-worker
        Cloudflare Worker
        Screenshot generation"]
    end

    subgraph Tooling ["tooling & ci/"]
        CI["ci/ — release automation
        reporters, scripts, utils"]
        LINT["lint/ — custom ESLint plugins
        enforce-variant-standard,
        no-primitive-colors, etc."]
        TOOLS["tools/deployments/
        PR description validator"]
    end

    ROOT --> KUMO
    ROOT --> DOCS
    ROOT --> FIGMA
    ROOT --> SCREENSHOT
    ROOT --> CI
    ROOT --> LINT
    ROOT --> TOOLS
    KUMO --> DOCS
    FIGMA --> KUMO
```

**Source:** `pnpm-workspace.yaml` at root defines workspace packages: `packages/*`.

### kumo Component Layer Architecture

```mermaid
flowchart TD
    subgraph "CSS Layer"
        TAILWIND["Tailwind CSS v4
        @theme directive, light-dark()"]
        KUMOCSS["kumo-standalone.css /
        kumo.css (semantic tokens)"]
    end

    subgraph "Primitives Layer"
        BASE["Base UI (@base-ui/react ^1.6.0)
        Unstyled accessible components"]
        PRIM["src/primitives/
        30+ primitive wrappers
        accordion, dialog, select, etc."]
    end

    subgraph "Components Layer"
        COMPS["src/components/
        35+ styled components
        Button, Dialog, Sidebar, etc."]
        BLOCKS["src/blocks/
        Composite page-level components
        DeleteResource"]
    end

    subgraph "Utilities"
        UTILS["src/utils/
        cn(), link-provider, portal-provider"]
        REGISTRY["src/registry/
        Component metadata, AI schemas"]
        CODE["src/code/
        Syntax highlighting (Shiki)"]
        CATALOG["src/catalog/
        Component catalog system"]
    end

    TAILWIND --> KUMOCSS
    BASE --> PRIM
    PRIM --> COMPS
    KUMOCSS --> COMPS
    UTILS --> COMPS
    UTILS --> PRIM
    COMPS --> BLOCKS
    REGISTRY --> COMPS
    CODE --> COMPS
```

**Aha:** The primitives layer is *generated*, not hand-written. Running `pnpm run codegen:primitives` executes `scripts/generate-primitives.ts` which creates primitive wrappers from a template. This means adding a new Base UI primitive to Kumo is a codegen step, not a manual implementation — the primitives stay in lockstep with Base UI's API.

### kumo Import Resolution

kumo uses granular `exports` in `package.json` (lines 26-411) to enable tree-shakable imports:

```
@cloudflare/kumo                     → dist/index.js (all exports)
@cloudflare/kumo/components/button   → dist/components/button.js (single component)
@cloudflare/kumo/primitives/dialog   → dist/primitives/dialog.js (primitive only)
@cloudflare/kumo/utils               → dist/utils.js (cn, link-provider)
@cloudflare/kumo/registry            → dist/registry.js (metadata)
@cloudflare/kumo/code                → dist/code.js (syntax highlighting)
@cloudflare/kumo/code/server       → dist/code/server.js (server-side rendering)
@cloudflare/kumo/styles              → dist/styles/kumo.css
@cloudflare/kumo/ai/schemas          → dist/ai/schemas.js (Zod schemas for AI)
```

This means importing `@cloudflare/kumo/components/button` only bundles the Button component, not the entire library.

### kumo Versioning

kumo uses [changesets](https://github.com/changesets/changesets) for version management. The `.changeset/` directory contains pending change descriptions:

| Changeset | Description |
|-----------|-------------|
| `blue-red-button-rings` | Button ring style update |
| `bright-toasts-close` | Toast component close behavior fix |
| `fix-button-firefox-turnstile` | Firefox Turnstile compatibility fix |
| `fix-dialog-size-width` | Dialog size width correction |
| `flow-vertical-orientation` | Flow component vertical orientation |
| `fresh-otters-draw` | (pending change) |
| `spicy-maps-glow` | (pending change) |

**Source:** `.changeset/` directory at kumo root.

## mapcn Architecture

### Registry Pattern

```mermaid
flowchart LR
    REGISTRY["registry.json
    shadcn schema
    8 items defined"]
    PAGES["src/app/
    Next.js App Router
    docs + demo pages"]
    BLOCKS["src/registry/blocks/
    analytics-map, choropleth,
    heatmap, logistics-network,
    store-locator, uptime-monitor,
    delivery-tracker"]
    COMPONENTS["src/components/ui/
    18 shadcn components
    button, dialog, sidebar, etc."]
    MAP["src/registry/map.tsx
    Core Map component
    MapLibre GL wrapper"]

    REGISTRY -->|defines| PAGES
    REGISTRY -->|defines| BLOCKS
    REGISTRY -->|references| COMPONENTS
    REGISTRY -->|references| MAP
    BLOCKS -->|uses| MAP
    BLOCKS -->|uses| COMPONENTS
```

**Source:** `registry.json` (345 lines, 8 items).

### mapcn Component Surface

The core `Map` component (`src/registry/map.tsx`) wraps MapLibre GL and exposes:

| Feature | Implementation |
|---------|---------------|
| Markers | `Marker` with custom elements, labels, tooltips, popups |
| Popups | Standalone and marker-attached popup composition |
| Routes | Line routes with optional markers and interaction handlers |
| Arcs | Curved connection arcs between coordinates |
| GeoJSON | Sources and layers for custom map data |
| Clusters | Clustered point layers for dense datasets |

**Source:** `registry.json` meta.components (lines 34-75).

### mapcn Dependencies

```
mapcn (Next.js 15 app)
├── maplibre-gl          (map rendering)
├── lucide-react         (icons)
├── cmdk                 (command palette / search)
├── radix-ui             (accessible primitives)
├── class-variance-authority (component variants)
├── clsx + tailwind-merge (class merging)
├── next-themes          (theme provider)
└── @vercel/analytics    (usage tracking)
```

**Source:** `mapcn/package.json`.

## next-maps Architecture

### Layer Diagram

```mermaid
flowchart TD
    subgraph "App Layer"
        PAGE["src/app/page.tsx
        Main demo page"]
        LAYOUT["src/app/layout.tsx
        Root layout with providers"]
    end

    subgraph "Provider Layer"
        THEME["src/components/theme-provider.tsx
        next-themes wrapper"]
        MAPCTX["src/context/map-context.ts
        React context for Mapbox instance"]
        MAPBOX["src/lib/mapbox/provider.tsx
        Mapbox lifecycle wrapper"]
    end

    subgraph "Map Component Layer"
        CONTROLS["src/components/map/map-controls.tsx
        Zoom, style, geolocation controls"]
        MARKER["src/components/map/map-marker.tsx
        Mapbox marker wrapper"]
        POPUP["src/components/map/map-popup.tsx
        Mapbox popup wrapper"]
        SEARCH["src/components/map/map-search.tsx
        Mapbox Searchbox integration"]
        STYLES["src/components/map/map-styles.tsx
        Style toggle (satellite, street, dark)"]
    end

    subgraph "UI Layer"
        SHADCN["src/components/ui/
        8 shadcn components:
        button, dialog, input,
        command, popover, badge,
        separator, tabs"]
    end

    subgraph "Utilities"
        DEBOUNCE["src/hooks/useDebounce.ts"]
        UTILS["src/lib/utils.ts (cn helper)"]
        API["src/lib/mapbox/api.ts"]
        CONSTS["src/lib/mapbox/constants.ts"]
        UTLX["src/lib/mapbox/utils.tsx"]
    end

    PAGE --> THEME
    PAGE --> MAPCTX
    PAGE --> CONTROLS
    PAGE --> SEARCH
    PAGE --> STYLES
    CONTROLS --> MAPBOX
    MARKER --> MAPBOX
    POPUP --> MAPBOX
    SEARCH --> API
    SEARCH --> DEBOUNCE
    MAPCTX --> MAPBOX
```

### Map Context API

The map context (`src/context/map-context.ts`) exposes the Mapbox `map` instance to all child components:

```tsx
// src/context/map-context.ts
// Provides: map instance, loading state, center/zoom access
```

The provider (`src/lib/mapbox/provider.tsx`) manages the Mapbox map lifecycle:
- Creates `new mapboxgl.Map()` with config
- Cleans up `map.remove()` on unmount
- Delegates events to child components

**Source:** `next-maps/src/context/map-context.ts`, `next-maps/src/lib/mapbox/provider.tsx`.

### Dependencies

```
next-maps
├── mapbox-gl            (map rendering — requires API key)
├── @radix-ui/react-*    (dialog, popover, separator, slot, tabs)
├── lucide-react         (icons)
├── cmdk                 (command palette)
├── class-variance-authority
├── clsx + tailwind-merge
└── @types/mapbox__mapbox-gl-geocoder
```

**Source:** `next-maps/package.json`.

## table-forge Architecture

### MDX-Driven Documentation

```mermaid
flowchart TD
    subgraph "Build"
        MDX["@next/mdx
        MDX → React components
        at build time"]
        TW["Tailwind CSS 4
        PostCSS processing"]
    end

    subgraph "Pages"
        HOME["src/app/page.tsx
        Gallery view of all tables"]
        EXAMPLES["src/components/TableExample.tsx
        Live preview wrapper"]
    end

    subgraph "Tables"
        BASIC["BasicBorderedTable.tsx"]
        SORT["SortableTable.tsx"]
        FILTER["FilterableTable.tsx"]
        PAG["PaginatedTable.tsx"]
        STRIPE["StripedRowsTable.tsx"]
        COMPACT["CompactTable.tsx"]
        ACTIONS["WithActionsTable.tsx"]
        MOBILE["ResponsiveCardsOnMobile.tsx"]
        MIN["MinimalistUI.tsx"]
        TOPCTL["TopControlsSortableTable.tsx"]
    end

    subgraph "MDX Examples"
        MDX1["BasicBordered.mdx"]
        MDX2["Sortable.mdx"]
        MDX3["Filterable.mdx"]
        MDX4["Pagination.mdx"]
        MDX5["etc..."]
    end

    subgraph "UI"
        SHADCN["src/components/ui/
        button, card, tabs"]
    end

    MDX --> MDX1
    MDX --> MDX2
    MDX --> MDX3
    MDX1 --> BASIC
    MDX2 --> SORT
    MDX3 --> FILTER
    HOME --> EXAMPLES
    EXAMPLES --> BASIC
    EXAMPLES --> SORT
    EXAMPLES --> FILTER
```

### Table Components

All table components live in `src/components/tables/` and are documented via corresponding MDX files in `src/examples/tables/`:

| Table Component | MDX File | Feature |
|----------------|----------|---------|
| `BasicBorderedTable.tsx` | `BasicBordered.mdx` | Simple bordered table |
| `SortableTable.tsx` | `Sortable.mdx` | Column sorting |
| `FilterableTable.tsx` | `Filterable.mdx` | Row filtering |
| `PaginatedTable.tsx` | `Pagination.mdx` | Pagination controls |
| `StripedRowsTable.tsx` | `StripedRows.mdx` | Alternating row colors |
| `CompactTable.tsx` | `Compact.mdx` | Dense layout |
| `WithActionsTable.tsx` | `WithActions.mdx` | Action buttons per row |
| `ResponsiveCardsOnMobile.tsx` | `ResponsiveCardsOnMobile.mdx` | Mobile card layout |
| `MinimalistUI.tsx` | `Minimalist.mdx` | Minimal design |
| `TopControlsSortableTable.tsx` | `TopControlsSortable.mdx` | Sort + search controls |

**Source:** `table-forge/src/components/tables/`, `table-forge/src/examples/tables/`.

### Dependencies

```
table-forge
├── next (15, App Router)
├── react (19)
├── tailwindcss (4)
├── @mdx-js/loader + @mdx-js/react + @next/mdx
├── lucide-react (icons)
├── class-variance-authority
├── clsx + tailwind-merge
├── @radix-ui/react-slot, react-tabs
└── @vercel/analytics + next-themes
```

**Source:** `table-forge/package.json`.

## Next

Continue to [02-kumo](02-kumo.md) for a deep dive into Kumo's component architecture, theming system, and AI registry.
