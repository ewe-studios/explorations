# mapcn Blocks: 7 Pre-Built Map Templates

7 pre-built map blocks installable via shadcn registry. Each is a complete page with data, components, and layout.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/mapcn/src/registry/blocks/`

## Block Catalog

| Block | Files | iframe Height | Description |
|-------|-------|--------------|-------------|
| `analytics-map` | 4 | 940px | Real-time analytics: world map + overview cards + device stats |
| `choropleth` | 3 | 720px | World choropleth: country shading by metric with hover tooltip |
| `uptime-monitor` | 4 | 720px | Status-page: edge network health with live latency and uptime |
| `logistics-network` | 4 | 800px | Domestic logistics: map with sidebar stats and filters |
| `store-locator` | 4 | 800px | Searchable store list synced with map markers |
| `heatmap` | 1 | 800px | Globe-projected earthquake density heatmap |
| `delivery-tracker` | 2 | 720px | Live order tracking: route progress + courier position |

## analytics-map

**Source:** `src/registry/blocks/analytics-map/`

```
analytics-map/
├── page.tsx              ← Analytics dashboard page
├── data.ts               ← Sample analytics data
└── components/
    ├── overview-card.tsx ← Overview stats card
    └── breakdown-card.tsx← Device breakdown card
```

Uses Recharts for charts and mapcn Map for world visualization.

## choropleth

**Source:** `src/registry/blocks/choropleth/`

```
choropleth/
├── page.tsx              ← Choropleth map page
└── data.ts               ← Country metric data
```

Uses `blank` prop on Map for transparent basemap, draws GeoJSON country polygons.

## uptime-monitor

**Source:** `src/registry/blocks/uptime-monitor/`

```
uptime-monitor/
├── page.tsx                          ← Status page
├── data.ts                           ← Edge node data
└── components/
    ├── edge-node-marker.tsx          ← Custom marker for nodes
    └── status-sidebar.tsx            ← Status sidebar
```

Color-coded markers show node health.

## logistics-network

**Source:** `src/registry/blocks/logistics-network/`

```
logistics-network/
├── page.tsx                              ← Logistics dashboard
├── data.ts                               ← Logistics data
└── components/
    ├── filter-sidebar.tsx                ← Filter controls
    └── network-map.tsx                   ← Network map
```

Uses MapRoute for shipping routes and MapMarker for warehouse locations.

## store-locator

**Source:** `src/registry/blocks/store-locator/`

```
store-locator/
├── page.tsx                          ← Store locator page
├── data.ts                           ← Store data
└── components/
    ├── store-list.tsx                ← Searchable store list
    └── locator-map.tsx               ← Map with markers
```

Store list and map markers are synced — selecting a store highlights its marker.

## heatmap

**Source:** `src/registry/blocks/heatmap/`

```
heatmap/
└── page.tsx              ← Single-file heatmap
```

Uses `{ type: "globe" }` projection and MapLibre's heatmap layer.

## delivery-tracker

**Source:** `src/registry/blocks/delivery-tracker/`

```
delivery-tracker/
├── page.tsx              ← Delivery tracking page
└── data.ts               ← Delivery route data
```

Uses MapRoute for delivery path with moving courier marker.

## Installation

```bash
npx shadcn@latest add "https://mapcn.dev/r/choropleth.json"
```

Each block copies source files into your project — you own the code.
