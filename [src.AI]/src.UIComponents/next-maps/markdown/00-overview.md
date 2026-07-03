# next-maps: Mapbox Showcase for Next.js

Mapbox GL JS integration with shadcn/ui components. Unlike mapcn (shadcn registry), this is a standalone application to fork and customize.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/next-maps/`

## Architecture

```
src/
├── context/map-context.ts     ← React context for map instance
├── lib/mapbox/
│   ├── provider.tsx           ← Mapbox lifecycle wrapper
│   ├── constants.ts           ← FLY_TO: zoom 14, speed 4, 1000ms
│   ├── api.ts                 ← Mapbox API wrapper
│   └── utils.tsx              ← Utilities
├── components/map/
│   ├── map-controls.tsx       ← Zoom, style toggle, geolocation
│   ├── map-marker.tsx         ← Custom marker wrapper
│   ├── map-popup.tsx          ← Popup wrapper
│   ├── map-search.tsx         ← Mapbox Searchbox integration
│   └── map-styles.tsx         ← Style selector
├── hooks/useDebounce.ts       ← 400ms debounce for search
└── components/ui/             ← 8 shadcn components
```

## Map Context

```typescript
interface MapContextType { map: mapboxgl.Map }
const MapContext = createContext<MapContextType | null>(null)
function useMap() {
  const context = useContext(MapContext)
  if (!context) throw new Error("useMap must be used within a MapProvider")
  return context
}
```

## Mapbox Provider

```typescript
// src/lib/mapbox/provider.tsx
mapboxgl.accessToken = process.env.NEXT_PUBLIC_MAPBOX_TOKEN!

const mapInstance = new mapboxgl.Map({
  container: mapContainerRef.current,
  style: "mapbox://styles/mapbox/standard",  // 3D Standard style
  center: [longitude, latitude],
  zoom: initialZoom,
  attributionControl: false,
  logoPosition: "bottom-right",
})
```

Uses `mapbox://styles/mapbox/standard` — the newer 3D Standard style (v2), not the flat 2D streets-v12.

## Constants

```typescript
export const MAP_CONSTANTS = {
  FLY_TO: { ZOOM: 14, SPEED: 4, DURATION: 1000 },
  SEARCH: {
    DEBOUNCE_MS: 400,
    DEFAULT_LIMIT: 5,
    DEFAULT_COUNTRY: "US",
    DEFAULT_PROXIMITY: [-122.4194, 37.7749],  // San Francisco
  },
}
```

## Search Integration

Search input → `useDebounce` (400ms) → Mapbox Searchbox API → results in command palette → user selects → `map.flyTo()` with constants.

## Dependencies

| Dependency | Purpose |
|-----------|---------|
| `mapbox-gl` | Map rendering (requires API key) |
| `@radix-ui/react-*` | Accessible primitives (dialog, popover, etc.) |
| `lucide-react` | Icons |
| `cmdk` | Command palette for search |
| `class-variance-authority` | Component variants |
