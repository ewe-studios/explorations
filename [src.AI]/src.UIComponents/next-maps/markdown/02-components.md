# Component Architecture

The app has 8 React components: 3 map primitives (Marker, Popup, Search), 2 map chrome (Controls, Styles), 2 wrappers (LocationMarker, LocationPopup), and 1 theme provider.

## MapSearch

Source: `src/components/map/map-search.tsx` — 272 lines (the largest component).

The search component is a command palette (`cmdk`) that searches locations via the Mapbox Search Box API and places markers on the map.

```mermaid
flowchart TD
    A[User types in search input] --> B["useDebounce(400ms)"]
    B --> C[searchLocations API call]
    C --> D{results?}
    D -->|Yes| E[show suggestions in CommandList]
    D -->|No| F["CommandEmpty: no results"]
    E --> G[User selects suggestion]
    G --> H[retrieveLocation API call]
    H --> I[LocationFeature received]
    I --> J[flyTo(location)]
    J --> K[place LocationMarker]
    K --> L[show LocationPopup on hover]
```

### State

| State | Type | Purpose |
|-------|------|---------|
| `query` | `string` | Current input value |
| `displayValue` | `string` | Shown text (may differ from query after selection) |
| `results` | `LocationSuggestion[]` | Search suggestions |
| `isSearching` | `boolean` | Loading indicator |
| `isOpen` | `boolean` | Command palette open/closed |
| `selectedLocation` | `LocationFeature \| null` | Currently selected location |
| `selectedLocations` | `LocationFeature[]` | All placed locations |
| `error` | `string \| null` | Error message |
| `hoveredLocation` | `LocationFeature \| null` | Location currently hovered |

### Search Flow

```tsx
// MapSearch uses useDebounce with MAP_CONSTANTS.SEARCH.DEBOUNCE_MS (400)
const debouncedQuery = useDebounce(query, MAP_CONSTANTS.SEARCH.DEBOUNCE_MS);

useEffect(() => {
  if (!debouncedQuery) { setResults([]); return; }
  const controller = new AbortController();
  searchLocations({
    query: debouncedQuery,
    country: MAP_CONSTANTS.SEARCH.DEFAULT_COUNTRY,
    limit: MAP_CONSTANTS.SEARCH.DEFAULT_LIMIT,
    proximity: MAP_CONSTANTS.SEARCH.DEFAULT_PROXIMITY,
    signal: controller.signal,
  }).then(setResults).catch(setError);
  return () => controller.abort();
}, [debouncedQuery]);
```

### Location Selection

When a user selects a suggestion:
1. Calls `retrieveLocation(suggestion.mapbox_id)` to get full GeoJSON
2. Flies the map to the location (`map.flyTo({ center, zoom: 14, duration: 1000 })`)
3. Adds the location to `selectedLocations`
4. Places a `LocationMarker` and shows `LocationPopup` on hover

## Marker

Source: `src/components/map/map-marker.tsx` — 105 lines.

A React Portal wrapper around `mapboxgl.Marker`:

```tsx
export default function Marker({ children, latitude, longitude, data, onHover, onClick, ...props }) {
  const { map } = useMap();
  const markerRef = useRef<mapboxgl.Marker>(null);
  const elRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!map) return;
    const el = document.createElement("div");
    elRef.current = el;
    const m = new mapboxgl.Marker({ element: el, ...props })
      .setLngLat([longitude, latitude])
      .addTo(map);
    markerRef.current = m;

    // Hover/click event listeners on the element
    el.addEventListener("mouseenter", ...);
    el.addEventListener("mouseleave", ...);
    el.addEventListener("click", ...);

    return () => { m.remove(); /* cleanup listeners */ };
  }, [map, latitude, longitude]);

  return elRef.current ? createPortal(children, elRef.current) : null;
}
```

**Aha:** The marker creates a `<div>` element, registers it with Mapbox via `new mapboxgl.Marker({ element: el })`, then uses `createPortal(children, el)` to render React children inside the Mapbox-managed DOM node. This lets React manage the visual content (the rose-colored circle with MapPin icon) while Mapbox handles the positioning and map synchronization.

## Popup

Source: `src/components/map/map-popup.tsx` — 83 lines.

Similar pattern to Marker but uses `mapboxgl.Popup`:

```tsx
export default function Popup({ children, latitude, longitude, marker, onClose, ...props }) {
  const container = useMemo(() => document.createElement("div"), []);

  useEffect(() => {
    if (!map) return;
    const popup = new mapboxgl.Popup({ ...props })
      .setDOMContent(container)
      .setLngLat([longitude, latitude]);
    if (marker) popup.setMarker(marker);
    popup.addTo(map);
    popup.on("close", handleClose);
    return () => popup.remove();
  }, [map, latitude, longitude]);

  return createPortal(children, container);
}
```

Same Portal pattern: React children rendered into Mapbox-managed popup DOM.

## LocationMarker

Source: `src/components/location-marker.tsx` — 30 lines.

A thin wrapper around `Marker` with specific styling:

```tsx
export function LocationMarker({ location, onHover }) {
  return (
    <Marker longitude={location.geometry.coordinates[0]}
            latitude={location.geometry.coordinates[1]}
            data={location} onHover={onHover}>
      <div className="rounded-full bg-rose-500 text-white shadow-lg size-8
                       cursor-pointer hover:scale-110">
        <MapPin className="stroke-[2.5px] size-4.5" />
      </div>
    </Marker>
  );
}
```

Rose-colored circle with MapPin icon, scales up on hover.

## LocationPopup

Source: `src/components/location-popup.tsx` — 112 lines.

A rich info card showing location details:

| Section | Data | Display |
|---------|------|---------|
| Name | `properties.name` | Bold heading |
| Status badge | `properties.operational_status` | "Open" (green) or status |
| Brand | `properties.brand[0]` | Subtitle if different from name |
| Address | `properties.full_address` | Truncated with MapPin icon |
| Categories | `properties.poi_category[]` | Up to 3 badges + "+N more" |
| Footer | `properties.mapbox_id`, coordinates | Shortened ID, lat/lng |

The popup icon is determined by looking up the POI category in `iconMap` (14 categories mapped to Lucide icons).

## MapStyles

Source: `src/components/map/map-styles.tsx` — 86 lines.

A tab-based style switcher with 5 options:

| Style ID | Label | Icon | Theme Effect |
|----------|-------|------|-------------|
| `streets-v12` | Map | MapIcon | Light mode |
| `satellite-streets-v12` | Satellite | SatelliteIcon | Light mode |
| `outdoors-v12` | Terrain | TreesIcon | Light mode |
| `light-v11` | Light | SunIcon | Light mode |
| `dark-v11` | Dark | MoonIcon | **Switches to dark theme** |

```tsx
// When "dark" style is selected, calls setTheme("dark")
// Any other style calls setTheme("light")
```

## MapControls

Source: `src/components/map/map-controls.tsx` — 30 lines.

Simple zoom in/out buttons, positioned at bottom-right:

```tsx
export default function MapCotrols() {
  const { map } = useMap();
  return (
    <aside className="absolute bottom-8 right-4 z-10 bg-background p-2 rounded-lg shadow-lg">
      <Button onClick={() => map?.zoomIn()}><PlusIcon /></Button>
      <Button onClick={() => map?.zoomOut()}><MinusIcon /></Button>
    </aside>
  );
}
```

Note the typo in the export name: `MapCotrols` (not `MapControls`).

## ThemeProvider

Source: `src/components/theme-provider.tsx` — 11 lines.

A thin passthrough to `next-themes`:

```tsx
export function ThemeProvider({ children, ...props }) {
  return <NextThemesProvider {...props}>{children}</NextThemesProvider>;
}
```

Used in `layout.tsx` with `attribute="class"`, `defaultTheme="system"`, `enableSystem`, `disableTransitionOnChange`.

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Map Context](01-map-context.md) — Provider, API, constants
- [App Structure](03-app-structure.md) — Layout, CSS, shadcn/ui
