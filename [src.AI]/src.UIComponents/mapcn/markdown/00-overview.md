# mapcn: shadcn-Style Map Components

MapLibre-powered map components installed via `npx shadcn add`. You own the code after installation.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/mapcn/`

## Map Component API

**Source:** `src/registry/map.tsx` (2192 lines)

```tsx
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
| `projection` | `ProjectionSpecification` | — | Map projection (e.g., `{ type: "globe" }`) |
| `viewport` | `Partial<MapViewport>` | — | Controlled viewport |
| `onViewportChange` | `(viewport) => void` | — | Viewport change callback |
| `loading` | `boolean` | false | Show loading indicator |

### Exports

| Export | Type |
|--------|------|
| `Map` | Main map component |
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
  // 3. Watch both for changes via MutationObserver + MediaQueryList
}
```

### Blank Mode

Transparent basemap for choropleths, arcs, dot maps:

```typescript
const blankMapStyle: MapLibreGL.StyleSpecification = {
  version: 8,
  sources: {},
  layers: [{ id: "background", type: "background", paint: { "background-color": "rgba(0,0,0,0)" } }],
};
```

## Pre-built Blocks

| Block | Files | Description |
|-------|-------|-------------|
| `map` | 1 | Base MapLibre component |
| `analytics-map` | 4 | World map + overview cards + device stats |
| `choropleth` | 3 | Country shading by metric with hover tooltip |
| `uptime-monitor` | 4 | Edge network health map with status sidebar |
| `logistics-network` | 4 | Domestic logistics map with filter sidebar |
| `store-locator` | 4 | Searchable store list synced to map markers |
| `heatmap` | 1 | Globe-projected earthquake density heatmap |
| `delivery-tracker` | 2 | Live order tracking with route progress |

## Installation

```bash
npx shadcn@latest add "https://mapcn.dev/r/map.json"
npx shadcn@latest add "https://mapcn.dev/r/choropleth.json"
```
