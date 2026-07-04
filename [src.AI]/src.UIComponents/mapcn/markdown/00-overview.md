# mapcn: shadcn-Style Map Components

shadcn/ui-style registry for MapLibre GL maps. 1 core component + 7 pre-built blocks. Install via `npx shadcn add` — you own the code.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/mapcn/`

## Package Info

| Attribute | Value |
|-----------|-------|
| Files | 88 source files |
| Map Engine | MapLibre GL |
| Type | shadcn registry (not npm package) |
| Blocks | 8 (1 core + 7 pre-built) |

## Registry Structure

**Source:** `registry.json` (345 lines, 8 items)

| Item | Type | Description |
|------|------|-------------|
| `map` | `registry:ui` | Core MapLibre component with markers, popups, tooltips, routes, arcs, GeoJSON, clusters |
| `analytics-map` | `registry:block` | World map + overview cards + device stats (4 files) |
| `choropleth` | `registry:block` | Country shading by metric with hover tooltip (3 files) |
| `uptime-monitor` | `registry:block` | Edge network health map with status sidebar (4 files) |
| `logistics-network` | `registry:block` | Domestic logistics map with filter sidebar (4 files) |
| `store-locator` | `registry:block` | Searchable store list synced to map markers (4 files) |
| `heatmap` | `registry:block` | Globe-projected earthquake density heatmap (1 file) |
| `delivery-tracker` | `registry:block` | Live order tracking with route progress (2 files) |

## Map Component API

**Source:** `src/registry/map.tsx` (2192 lines)

```tsx
import { Map, MapMarker, MapControls, MapRoute } from '@/registry/map'

<Map theme="dark" blank={false}>
  <MapMarker lat={37.7749} lng={-122.4194} label="SF" />
  <MapControls />
  <MapRoute coordinates={[[lon1,lat1],[lon2,lat2]]} />
</Map>
```

### Props

| Prop | Type | Default | Purpose |
|------|------|---------|---------|
| `theme` | `"light" \| "dark"` | Auto-detect | Map style theme |
| `styles` | `{ light?, dark? }` | Carto basemaps | Custom map styles |
| `blank` | `boolean` | false | Transparent basemap for data viz |
| `projection` | `ProjectionSpecification` | — | Map projection (`{ type: "globe" }`) |
| `viewport` | `Partial<MapViewport>` | — | Controlled viewport |
| `onViewportChange` | `(viewport) => void` | — | Viewport change callback |
| `loading` | `boolean` | false | Show loading indicator |

### Exports

| Export | Type |
|--------|------|
| `Map` | Main component |
| `useMap` | Hook for map instance |
| `MapMarker` | Marker with custom content |
| `MarkerContent` | Marker body |
| `MarkerPopup` | Popup attached to marker |
| `MarkerTooltip` | Tooltip attached to marker |
| `MarkerLabel` | Text label |
| `MapPopup` | Standalone popup |
| `MapControls` | Zoom, geolocate, fullscreen, compass |
| `MapRoute` | Line route |
| `MapArc` | Curved arc |
| `MapGeoJSON` | GeoJSON source + layer |
| `MapClusterLayer` | Clustered points |

### Theme Detection

```typescript
function useResolvedTheme(themeProp?: "light" | "dark"): Theme {
  // 1. Check document class (next-themes)
  // 2. Fall back to system preference (prefers-color-scheme)
  // 3. Watch both via MutationObserver + MediaQueryList
}
```

### Default Styles

```typescript
const defaultStyles = {
  dark: "https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json",
  light: "https://basemaps.cartocdn.com/gl/positron-gl-style/style.json",
};
```

### Blank Mode

```typescript
const blankMapStyle: MapLibreGL.StyleSpecification = {
  version: 8,
  sources: {},
  layers: [{ id: "background", type: "background", paint: { "background-color": "rgba(0,0,0,0)" } }],
};
```

Transparent basemap for choropleths, arcs, dot maps.

## Installation

```bash
npx shadcn@latest add "https://mapcn.dev/r/map.json"
npx shadcn@latest add "https://mapcn.dev/r/choropleth.json"
```

## Documentation Site

**Source:** `src/app/(main)/docs/`

| Page | Purpose |
|------|---------|
| `/docs/installation` | Install guide |
| `/docs/basic-map` | Basic map usage |
| `/docs/markers` | Marker usage |
| `/docs/popups` | Popup usage |
| `/docs/routes` | Route usage |
| `/docs/arcs` | Arc usage |
| `/docs/geojson` | GeoJSON usage |
| `/docs/clusters` | Cluster usage |
| `/docs/controls` | Controls usage |
| `/docs/advanced-usage` | Advanced patterns |
| `/docs/api-reference` | Full API docs |

## Next

Continue to [01-blocks](01-blocks.md) for the 7 pre-built block details.
