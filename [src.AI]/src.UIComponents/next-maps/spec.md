---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/next-maps/
language: TypeScript/React
framework: Next.js 15.2.3 + React 19
explored_at: 2026-07-04
license: MIT
---

# spec.md -- next-maps Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/next-maps/`
- **Language:** TypeScript/React
- **Framework:** Next.js 15.2.3 (App Router, Turbopack)
- **Source files:** 18 (excluding node_modules, .git, public assets)
- **License:** MIT

## What next-maps Is

next-maps is a full-screen interactive map application built with Next.js 15 and Mapbox GL JS. It provides location search via the Mapbox Search Box API (suggest + retrieve endpoints), displays search results as custom React-rendered markers with hover-aware popup cards, supports five switchable map styles (streets, satellite, terrain, light, dark), automatically syncs the UI theme (light/dark) to the selected map style, and renders everything through a React Context-based architecture that bridges React Portal rendering with Mapbox's imperative DOM markers and popups.

## Documentation Goal

A reader should understand:
1. How the MapContext bridges React's declarative model with Mapbox GL JS's imperative API
2. How the two-phase Mapbox Search Box flow (suggest then retrieve) powers location discovery
3. How React Portals render JSX inside Mapbox-managed DOM elements
4. How the component layer (marker, popup, search, controls, styles) composes together
5. How the theme system automatically follows map style selection
6. The full Next.js app structure, styling pipeline, and build configuration

## Documentation Structure

```
next-maps/
  spec.md
  build.py
  markdown/
    README.md              -- Index / table of contents
    00-overview.md         -- What it is, architecture, Mapbox integration, quick start
    01-map-context.md      -- Map context state management, MapProvider, debouncing, search flow
    02-components.md       -- Marker, Popup, Search, Controls, Styles, Theme components
    03-app-structure.md    -- Next.js routing, layout, globals.css, shadcn/ui, build system
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | spec.md | DONE |
| 2 | 00-overview.md | DONE |
| 3 | 01-map-context.md | DONE |
| 4 | 02-components.md | DONE |
| 5 | 03-app-structure.md | DONE |
| 6 | README.md | DONE |
| 7 | HTML build | TODO |
| 8 | Grandfather review | TODO |

## Build System

```bash
cd "[src.AI]/src.UIComponents/next-maps" && python3 build.py
```

Zero dependencies. Python 3.12+ stdlib only. Converts markdown to HTML with embedded Mermaid, dark/light theme, prev/next navigation.

## Quality Requirements

All Iron Rules from the documentation directive apply: names match source, numbers match source, flows match execution, full API coverage, minimum 2 mermaid diagrams per document, minimum 1 Aha moment per document, cross-references, source path references.

## Expected Outcome

A developer can read these docs and understand the entire next-maps architecture -- from Next.js layout through Mapbox GL initialization, context-based map access, two-phase location search, React Portal marker/popup rendering, and theme synchronization -- without opening a single source file.
