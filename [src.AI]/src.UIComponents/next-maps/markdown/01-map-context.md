# Map Context and Provider Deep Dive

The Mapbox integration uses a React Context pattern to bridge React's declarative model with Mapbox's imperative API. This document traces every file in the `lib/mapbox/` directory and the context/hook layers.

## MapContext

Source: `src/context/map-context.ts` — 15 lines.

```typescript
interface MapContextType {
  map: mapboxgl.Map;
}

export const MapContext = createContext<MapContextType | null>(null);

export function useMap() {
  const context = useContext(MapContext);
  if (!context) {
    throw new Error("useMap must be used within a MapProvider");
  }
  return context;
}
```

A minimal context: just the `mapboxgl.Map` instance. The `useMap` hook enforces that it's called within a `MapProvider` — throwing prevents silent failures when a child component tries to use the map before it's initialized.

## MapProvider

Source: `src/lib/mapbox/provider.tsx` — 75 lines.

```tsx
// src/lib/mapbox/provider.tsx:10
mapboxgl.accessToken = process.env.NEXT_PUBLIC_MAPBOX_TOKEN!;
```

The token is set at module scope (outside the component), so it's assigned once during module initialization, not on every render.

### Map Creation

```tsx
// src/lib/mapbox/provider.tsx:38-50
const mapInstance = new mapboxgl.Map({
  container: mapContainerRef.current,
  style: "mapbox://styles/mapbox/standard",
  center: [initialViewState.longitude, initialViewState.latitude],
  zoom: initialViewState.zoom,
  attributionControl: false,
  logoPosition: "bottom-right",
});
```

Key choices:
- **`attributionControl: false`** — Hides the default Mapbox attribution; the app handles attribution itself.
- **`logoPosition: "bottom-right"`** — Moves the Mapbox logo away from where UI controls are placed.
- **Style: `"mapbox://styles/mapbox/standard"`** — Uses the new Standard style (3D buildings, terrain).

### Cleanup

```tsx
return () => {
  mapInstance.off("load", onLoad);
  mapInstance.remove();
  mapRef.current = null;
};
```

The cleanup function removes the event listener, destroys the map instance, and nulls the ref. This prevents memory leaks and duplicate map instances on hot reload.

### Loading State

```tsx
{!loaded && (
  <div className="absolute inset-0 flex items-center justify-center bg-background/80 z-[1000]">
    <Loader className="size-5 animate-spin" />
    Loading map...
  </div>
)}
```

A full-screen overlay with a spinner, removed when the `"load"` event fires.

## Mapbox API

Source: `src/lib/mapbox/api.ts` — 112 lines.

### Session Token

```ts
// src/lib/mapbox/api.ts:5-15
function getSessionToken(): string {
  let token = sessionStorage.getItem("mapbox_session_token");
  if (!token) {
    token = process.env.NEXT_PUBLIC_MAPBOX_SESSION_TOKEN || "";
    sessionStorage.setItem("mapbox_session_token", token);
  }
  return token;
}
```

Caches the session token in `sessionStorage`. Groups search + retrieve calls into a single Mapbox billing session.

### searchLocations

```ts
// src/lib/mapbox/api.ts:33-60
export async function searchLocations(options: SearchOptions): Promise<LocationSuggestion[]> {
  const { query, country = "US", limit = 5, proximity, signal } = options;
  // GET /search/searchbox/v1/suggest
}
```

| Parameter | Default | Description |
|-----------|---------|-------------|
| `query` | (required) | Search text |
| `country` | `"US"` | Country filter (ISO 3166-1 alpha-2) |
| `limit` | `5` | Max suggestions |
| `proximity` | `[-122.4194, 37.7749]` | San Francisco for proximity bias |
| `signal` | `AbortSignal` | Request cancellation |

### retrieveLocation

```ts
// src/lib/mapbox/api.ts:62-112
export async function retrieveLocation(mapboxId: string): Promise<LocationFeature[]> {
  // GET /search/searchbox/v1/retrieve/{mapboxId}
}
```

Takes a `mapbox_id` from a suggestion and returns the full GeoJSON Feature with geometry and properties.

## Constants

Source: `src/lib/mapbox/constants.ts` — 13 lines.

```ts
export const MAP_CONSTANTS = {
  FLY_TO: { ZOOM: 14, SPEED: 4, DURATION: 1000 },
  SEARCH: { DEBOUNCE_MS: 400, DEFAULT_LIMIT: 5, DEFAULT_COUNTRY: "US",
    DEFAULT_PROXIMITY: [-122.4194, 37.7749] as [number, number] },
};
```

| Constant | Value | Used When |
|----------|-------|-----------|
| `FLY_TO.ZOOM` | 14 | User selects a search result |
| `FLY_TO.SPEED` | 4 | Fly animation speed |
| `FLY_TO.DURATION` | 1000ms | Fly animation duration |
| `SEARCH.DEBOUNCE_MS` | 400 | Search input debounce |
| `SEARCH.DEFAULT_LIMIT` | 5 | Max search results |
| `SEARCH.DEFAULT_COUNTRY` | `"US"` | Country filter |
| `SEARCH.DEFAULT_PROXIMITY` | SF coordinates | Proximity bias |

## useDebounce Hook

Source: `src/hooks/useDebounce.ts` — 12 lines.

```ts
export function useDebounce<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const handler = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(handler);
  }, [value, delay]);
  return debounced;
}
```

Generic debounce hook. Used in `MapSearch` to debounce search API calls by 400ms (from `MAP_CONSTANTS.SEARCH.DEBOUNCE_MS`). Each time `value` changes, the previous timeout is cleared and a new one starts.

## cn() Utility

Source: `src/lib/utils.ts` — 6 lines.

```ts
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

Combines `clsx` (conditional class names) with `tailwind-merge` (resolves Tailwind class conflicts). Used throughout all components.

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Components](02-components.md) — Marker, Popup, Search, Controls, Styles
- [App Structure](03-app-structure.md) — Next.js layout, CSS, shadcn/ui
