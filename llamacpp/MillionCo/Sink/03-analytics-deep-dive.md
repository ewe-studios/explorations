---
title: "Analytics Engine Deep Dive"
subtitle: "Real-time click tracking, aggregation, and 3D globe visualization"
---

# Analytics Engine Deep Dive

## Overview

Sink uses Cloudflare Analytics Engine to track and visualize click events in real-time. This document covers:
- Event ingestion pipeline
- SQL querying patterns
- Real-time aggregation
- 3D globe visualization data flow

---

## Chapter 1: Analytics Engine Architecture

### What is Analytics Engine?

Analytics Engine is Cloudflare's time-series database optimized for:
- **High write throughput** - Millions of events per second
- **Real-time queries** - Sub-second aggregation
- **Edge location** - Queries run near your data

### Data Model

```
┌─────────────────────────────────────────────────────────┐
│                    DataPoint                            │
├─────────────────────────────────────────────────────────┤
│  Blobs (8x string)  │  Doubles (8x number)  │  Indexes │
│  - blob1: slug      │  - double1: value     │  - slug  │
│  - blob2: country   │  - double2: latency   │  - date  │
│  - blob3: city      │  - double3: size      │          │
│  - blob4: device    │                       │          │
│  - blob5: browser   │                       │          │
│  - blob6: os        │                       │          │
│  - blob7: referrer  │                       │          │
│  - blob8: continent │                       │          │
└─────────────────────────────────────────────────────────┘
```

### Writing Events

```typescript
// server/utils/analytics/track.ts
import type { ClickEvent } from '~/shared/schemas/analytics'

export async function trackClick(
  request: Request,
  slug: string,
  responseTime: number = 0
): Promise<void> {
  const userAgent = request.headers.get('user-agent') || ''
  const referer = request.headers.get('referer') || ''

  // Parse user agent
  const ua = parseUserAgent(userAgent)

  // Get geo data from Cloudflare headers
  const geo = {
    country: request.headers.get('CF-IPCountry') || 'unknown',
    city: request.headers.get('CF-IPCity') || 'unknown',
    continent: request.headers.get('CF-IPContinent') || 'unknown',
    latitude: request.headers.get('CF-IPLatitude') || '0',
    longitude: request.headers.get('CF-IPLongitude') || '0'
  }

  const event: ClickEvent = {
    // Blobs - for grouping in SELECT/GROUP BY
    blob1: slug,
    blob2: geo.country,
    blob3: geo.city,
    blob4: geo.continent,
    blob5: ua.device,      // 'desktop', 'mobile', 'tablet'
    blob6: ua.browser,     // 'Chrome', 'Firefox', 'Safari'
    blob7: ua.os,          // 'Windows', 'macOS', 'iOS'
    blob8: truncate(referer, 512),

    // Doubles - for SUM/AVG aggregations
    double1: responseTime,
    double2: 0,  // Reserved for future use
    double3: 0,

    // Indexes - for WHERE clause filtering
    indexes: [slug, geo.country, geo.continent]
  }

  // Write to Analytics Engine
  ANALYTICS.writeDataPoint(event)
}

function parseUserAgent(ua: string) {
  // Simple UA parsing (in production, use ua-parser-js)
  const isMobile = /mobile|android|iphone|ipad/i.test(ua)

  return {
    device: isMobile ? 'mobile' : 'desktop',
    browser: getBrowserName(ua),
    os: getOSName(ua)
  }
}
```

---

## Chapter 2: Query Patterns

### Basic Aggregation

```typescript
// server/utils/analytics/queries.ts

// Total clicks for a slug
export async function getTotalClicks(
  slug: string,
  days: number = 7
): Promise<number> {
  const query = `
    SELECT SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${days}' DAY
  `

  const result = await ANALYTICS.query(query)
  return result[0]?.clicks || 0
}

// Clicks by country
export async function getClicksByCountry(
  slug: string,
  days: number = 7
): Promise<Array<{ country: string; clicks: number }>> {
  const query = `
    SELECT
      blob2 as country,
      SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${days}' DAY
    GROUP BY blob2
    ORDER BY clicks DESC
    LIMIT 50
  `

  return ANALYTICS.query(query)
}

// Clicks by device type
export async function getClicksByDevice(
  slug: string,
  days: number = 7
): Promise<Array<{ device: string; clicks: number }>> {
  const query = `
    SELECT
      blob5 as device,
      SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${days}' DAY
    GROUP BY blob5
    ORDER BY clicks DESC
  `

  return ANALYTICS.query(query)
}
```

### Time Series Data

```typescript
// Time series with dynamic granularity
export async function getTimeSeries(
  slug: string,
  days: number = 7
): Promise<Array<{ timestamp: string; clicks: number }>> {
  // Choose granularity based on time range
  const granularity = days <= 1 ? 'hour' : 'day'

  const query = `
    SELECT
      DATE_TRUNC('${granularity}', timestamp) as time,
      SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${days}' DAY
    GROUP BY 1
    ORDER BY 1
  `

  return ANALYTICS.query(query)
}

// Real-time last 24 hours (hourly)
export async function getRealtimeData(
  slug: string
): Promise<Array<{ hour: string; clicks: number }>> {
  const query = `
    SELECT
      DATE_FORMAT(timestamp, '%Y-%m-%d %H:00') as hour,
      SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '24' HOUR
    GROUP BY 1
    ORDER BY 1
  `

  return ANALYTICS.query(query)
}
```

### Geo Visualization Data

```typescript
// Data for 3D globe visualization
export interface GeoPoint {
  country: string
  latitude: number
  longitude: number
  clicks: number
}

export async function getGeoData(
  slug: string,
  days: number = 7
): Promise<GeoPoint[]> {
  // Country to coordinates mapping
  const countryCoords: Record<string, { lat: number; lng: number }> = {
    US: { lat: 37.0902, lng: -95.7129 },
    CN: { lat: 35.8617, lng: 104.1954 },
    DE: { lat: 51.1657, lng: 10.4515 },
    GB: { lat: 55.3781, lng: -3.4360 },
    // ... more countries
  }

  const query = `
    SELECT
      blob2 as country,
      SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${days}' DAY
    GROUP BY blob2
    HAVING clicks > 0
  `

  const results = await ANALYTICS.query(query)

  return results
    .filter((r: any) => countryCoords[r.country])
    .map((r: any) => ({
      country: r.country,
      latitude: countryCoords[r.country].lat,
      longitude: countryCoords[r.country].lng,
      clicks: r.clicks
    }))
}
```

---

## Chapter 3: Advanced Queries

### Referrer Analysis

```typescript
export async function getReferrers(
  slug: string,
  days: number = 7
): Promise<Array<{ referrer: string; clicks: number }>> {
  const query = `
    SELECT
      blob8 as referrer,
      SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${days}' DAY
      AND blob8 != 'direct'
      AND blob8 != ''
    GROUP BY blob8
    ORDER BY clicks DESC
    LIMIT 20
  `

  return ANALYTICS.query(query)
}
```

### Browser/OS Breakdown

```typescript
export async function getBrowserStats(
  slug: string,
  days: number = 7
): Promise<Array<{ browser: string; clicks: number; percentage: number }>> {
  const query = `
    WITH total AS (
      SELECT SUM(_sample_interval) as total
      FROM sink_analytics
      WHERE blob1 = '${slug}'
        AND timestamp > NOW() - INTERVAL '${days}' DAY
    ),
    by_browser AS (
      SELECT
        blob6 as browser,
        SUM(_sample_interval) as clicks
      FROM sink_analytics
      WHERE blob1 = '${slug}'
        AND timestamp > NOW() - INTERVAL '${days}' DAY
      GROUP BY blob6
      ORDER BY clicks DESC
      LIMIT 10
    )
    SELECT
      b.browser,
      b.clicks,
      ROUND(b.clicks * 100.0 / t.total, 2) as percentage
    FROM by_browser b, total t
  `

  return ANALYTICS.query(query)
}
```

### Peak Hours Analysis

```typescript
export async function getPeakHours(
  slug: string,
  days: number = 7
): Promise<Array<{ hour: number; clicks: number }>> {
  const query = `
    SELECT
      EXTRACT(HOUR FROM timestamp) as hour,
      SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${days}' DAY
    GROUP BY 1
    ORDER BY 2 DESC
    LIMIT 24
  `

  return ANALYTICS.query(query)
}
```

---

## Chapter 4: Real-Time Dashboard

### WebSocket for Live Updates

```typescript
// server/api/analytics/live.get.ts
import { nanoid } from 'nanoid'

export default eventHandler(async (event) => {
  const { slug } = event.context.params

  // For real-time updates, we poll Analytics Engine
  // In production, consider using Cloudflare Durable Objects
  // for true real-time websocket connections

  const sessionId = nanoid()

  // Return initial data
  const analytics = await getAnalytics(slug, '24h')

  return {
    sessionId,
    data: analytics,
    refreshInterval: 5000 // Client should poll every 5 seconds
  }
})
```

### Caching Strategy

```typescript
// server/utils/analytics/cache.ts
interface CachedAnalytics {
  data: any
  timestamp: number
  ttl: number
}

const cache = new Map<string, CachedAnalytics>()

export async function getCachedAnalytics(
  slug: string,
  range: string
): Promise<any> {
  const key = `${slug}:${range}`
  const cached = cache.get(key)

  if (cached && Date.now() - cached.timestamp < cached.ttl) {
    return cached.data
  }

  // Fresh query
  const data = await getAnalytics(slug, range as any)

  cache.set(key, {
    data,
    timestamp: Date.now(),
    ttl: range === '24h' ? 60000 : 300000 // 1min for 24h, 5min for others
  })

  return data
}
```

---

## Chapter 5: 3D Globe Visualization

### Data Structure for Globe

```typescript
// app/components/analytics/GlobeVisualization.vue
<script setup lang="ts">
import { ref, watch } from 'vue'
import type { GeoPoint } from '~/shared/schemas/analytics'

interface GlobePoint {
  lat: number
  lng: number
  value: number
  color: string
}

const props = defineProps<{
  slug: string
  days: number
}>()

const points = ref<GlobePoint[]>([])
const rotation = ref({ x: 0, y: 0 })

// Fetch geo data
async function fetchGeoData() {
  const response = await fetch(`/api/analytics/${props.slug}/geo?days=${props.days}`)
  const data = await response.json()

  points.value = data.map((point: GeoPoint) => ({
    lat: point.latitude,
    lng: point.longitude,
    value: point.clicks,
    color: getHeatColor(point.clicks)
  }))
}

// Heat color scale
function getHeatColor(value: number): string {
  const max = Math.max(...points.value.map(p => p.value))
  const normalized = value / max

  // Blue -> Green -> Yellow -> Red
  if (normalized < 0.25) {
    return `rgb(0, ${Math.floor(normalized * 4 * 255)}, 255)`
  } else if (normalized < 0.5) {
    return `rgb(0, 255, ${Math.floor((1 - (normalized - 0.25) * 4) * 255)})`
  } else if (normalized < 0.75) {
    return `rgb(${Math.floor((normalized - 0.5) * 4 * 255)}, 255, 0)`
  } else {
    return `rgb(255, ${Math.floor((1 - (normalized - 0.75) * 4) * 255)}, 0)`
  }
}

watch(() => [props.slug, props.days], fetchGeoData, { immediate: true })
</script>

<template>
  <div class="globe-container">
    <!-- Render 3D globe with points using Three.js or similar -->
  </div>
</template>
```

### Animation Loop

```typescript
// app/lib/globe-animation.ts
export function animateGlobe(
  globe: THREE.Group,
  points: THREE.Points[]
) {
  let lastTime = 0

  function animate(currentTime: number) {
    requestAnimationFrame(animate)

    const delta = currentTime - lastTime
    lastTime = currentTime

    // Slow rotation
    globe.rotation.y += 0.0005 * delta

    // Pulse effect for high-traffic points
    points.forEach(point => {
      point.scale.setScalar(1 + Math.sin(currentTime * 0.002) * 0.1)
    })

    renderer.render(scene, camera)
  }

  requestAnimationFrame(animate)
}
```

---

## Chapter 6: Performance Optimization

### Query Optimization

```typescript
// Use indexes efficiently
export async function optimizedQuery(
  slug: string,
  startDate: Date,
  endDate: Date
) {
  // GOOD: Uses indexes for filtering
  const query = `
    SELECT blob2, SUM(_sample_interval)
    FROM sink_analytics
    WHERE blob1 = '${slug}'           -- indexed
      AND timestamp >= ${startDate.getTime()}
      AND timestamp <= ${endDate.getTime()}
    GROUP BY blob2
  `

  // BAD: Full table scan
  // const badQuery = `
  //   SELECT blob2, SUM(_sample_interval)
  //   FROM sink_analytics
  //   WHERE blob2 = 'US'  -- Not in indexes array!
  //   GROUP BY blob2
  // `
}
```

### Sampling for Large Datasets

```typescript
// Analytics Engine uses sampling internally
// _sample_interval tells you how many events each row represents

export async function getEstimatedTotal(
  slug: string,
  days: number = 7
): Promise<number> {
  const query = `
    SELECT SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${days}' DAY
  `

  const result = await ANALYTICS.query(query)
  return result[0]?.clicks || 0
}
```

---

## Summary

Sink's analytics implementation:

1. **Event ingestion** - WriteDataPoint with blobs, doubles, indexes
2. **Query patterns** - Aggregation, time series, geo data
3. **Real-time visualization** - 3D globe with live updates
4. **Caching** - Client-side and edge caching
5. **Performance** - Indexed queries, sampling

---

## Next Steps

See [rust-revision.md](./rust-revision.md) for implementing analytics in Rust with valtron and the Analytics Engine HTTP API.
