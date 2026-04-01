---
title: "Storage Engine Deep Dive"
subtitle: "Workers KV data modeling, indexing strategies, and R2 storage"
---

# Storage Engine Deep Dive

## Overview

Sink uses a multi-tier storage architecture on Cloudflare:

1. **Workers KV** - Primary link storage with eventual consistency
2. **Analytics Engine** - Click events and real-time analytics
3. **R2** - Image uploads and QR code storage

This document covers the complete storage architecture, from data modeling to query optimization.

---

## Chapter 1: Workers KV Fundamentals

### What Makes KV Different?

Workers KV is **not** a traditional database. Understanding its characteristics is crucial:

| Property | Behavior | Implication |
|----------|----------|-------------|
| **Consistency** | Eventual (60s propagation) | Can't rely on immediate reads after writes |
| **Location** | Edge-cached | Reads are fast, writes are slower |
| **Operations** | Get/Put/List/Delete | No complex queries, joins, or transactions |
| **Value Size** | Up to 25MB | Good for JSON documents |
| **Key Size** | Up to 512 bytes | Design keys carefully |

### Key-Value Design Patterns

#### Pattern 1: Namespaced Keys

```typescript
// Bad: Flat namespace
await KV.put('abc123', linkData)

// Good: Namespaced keys
await KV.put('link:abc123', linkData)
await KV.put('user:user123:links:abc123', 'abc123')
```

**Why namespacing matters:**
- `KV.list({ prefix: 'link:' })` - Get all links
- `KV.list({ prefix: 'user:user123:links:' })` - Get user's links
- Avoids key collisions

#### Pattern 2: Composite Keys

```typescript
// Time-based listing
const timestamp = Date.now()
await KV.put(`user:${userId}:links:time:${timestamp}:${slug}`, slug)

// Alphabetical listing
await KV.put(`user:${userId}:links:alpha:${slug}`, slug)

// List by time (newest first)
const keys = await KV.list({
  prefix: `user:${userId}:links:time:`,
  reverse: true
})
```

#### Pattern 3: Index Separation

```typescript
// Primary data
await KV.put(`link:${slug}`, JSON.stringify({
  id: 'abc123',
  slug: 'abc123',
  url: 'https://example.com',
  userId: 'user123',
  createdAt: 1711612800000
}))

// Index for user listing
await KV.put(`idx:user:user123:link:${slug}`, '1')

// Index for analytics
await KV.put(`idx:slug:${slug}:clicks`, '0')
```

---

## Chapter 2: Link Storage Schema

### Primary Link Schema

```typescript
// shared/schemas/link.ts
import { z } from 'zod'

export const LinkSchema = z.object({
  // Core fields
  id: z.string().trim().max(26),           // nanoid(10) base64
  slug: z.string().trim().max(2048),       // Custom or generated
  url: z.string().trim().url().max(2048),  // Target URL

  // Ownership
  userId: z.string().optional(),           // Owner (if authenticated)

  // Timestamps
  createdAt: z.number(),                   // Unix ms
  updatedAt: z.number().optional(),
  expiresAt: z.number().optional(),        // Auto-delete after

  // Security
  password: z.string().optional(),         // Password protected
  isPrivate: z.boolean().default(false),   // Hidden from listing

  // Device routing
  deviceRouting: z.object({
    ios: z.string().url().optional(),
    android: z.string().url().optional()
  }).optional(),

  // OpenGraph preview
  og: z.object({
    title: z.string().max(512),
    description: z.string().max(1024),
    image: z.string().url().optional()
  }).optional(),

  // Metadata
  tags: z.array(z.string()).default([]),
  notes: z.string().max(4096).optional(),

  // Status
  isActive: z.boolean().default(true)
})

export type Link = z.infer<typeof LinkSchema>
```

### Storage Operations

#### Create Link

```typescript
// server/utils/link-storage.ts
import type { Link } from '~/shared/schemas/link'

export async function createLink(link: Link): Promise<void> {
  const key = `link:${link.slug}`
  const value = JSON.stringify(link)

  // Check for collision
  const existing = await KV.get(key)
  if (existing) {
    throw new Error(`Slug "${link.slug}" already exists`)
  }

  // Store with metadata
  await KV.put(key, value, {
    expirationTtl: link.expiresAt
      ? Math.floor((link.expiresAt - Date.now()) / 1000)
      : undefined,
    metadata: {
      userId: link.userId,
      createdAt: link.createdAt,
      updatedAt: Date.now()
    }
  })

  // Update user index
  if (link.userId) {
    await KV.put(
      `idx:user:${link.userId}:link:${link.slug}`,
      '1',
      { expirationTtl: 86400 * 30 } // 30 days
    )
  }
}
```

#### Get Link

```typescript
export async function getLink(slug: string): Promise<Link | null> {
  const key = `link:${slug}`
  const value = await KV.get(key)

  if (!value) return null

  const link = JSON.parse(value)

  // Check expiration
  if (link.expiresAt && link.expiresAt < Date.now()) {
    await KV.delete(key)
    return null
  }

  // Check active status
  if (!link.isActive) {
    return null
  }

  return link
}
```

#### Update Link

```typescript
export async function updateLink(
  slug: string,
  updates: Partial<Link>
): Promise<Link> {
  const key = `link:${slug}`
  const existing = await KV.get(key)

  if (!existing) {
    throw new Error('Link not found')
  }

  const link = JSON.parse(existing)
  const updated = { ...link, ...updates, updatedAt: Date.now() }

  await KV.put(key, JSON.stringify(updated), {
    metadata: {
      ...link.metadata,
      updatedAt: Date.now()
    }
  })

  return updated
}
```

#### Delete Link

```typescript
export async function deleteLink(slug: string): Promise<void> {
  const key = `link:${slug}`
  const existing = await KV.get(key)

  if (!existing) return

  const link = JSON.parse(existing)

  // Delete primary key
  await KV.delete(key)

  // Delete indexes
  if (link.userId) {
    await KV.delete(`idx:user:${link.userId}:link:${slug}`)
  }

  // Delete analytics index
  await KV.delete(`idx:slug:${slug}:clicks`)

  // Delete R2 assets
  await R2.delete(`qr:${slug}.png`)
}
```

---

## Chapter 3: Listing and Pagination

### Cursor-Based Pagination

```typescript
// server/utils/listing.ts
export interface ListOptions {
  userId?: string
  limit?: number
  cursor?: string
  search?: string
  sortBy?: 'createdAt' | 'slug' | 'clicks'
  sortOrder?: 'asc' | 'desc'
}

export async function listLinks(
  options: ListOptions
): Promise<{ links: Link[], cursor?: string, hasMore: boolean }> {
  const {
    userId,
    limit = 20,
    cursor,
    search,
    sortBy = 'createdAt',
    sortOrder = 'desc'
  } = options

  // Build prefix
  let prefix = 'link:'
  if (userId) {
    prefix = `idx:user:${userId}:link:`
  }

  // List with cursor
  const result = await KV.list({
    prefix,
    cursor,
    limit: limit + 1 // Get one extra to check hasMore
  })

  // Fetch full data for each key
  const links: Link[] = []
  for (const key of result.keys) {
    const value = await KV.get(key.name)
    if (value) {
      const link = JSON.parse(value)

      // Filter by search
      if (search && !link.slug.includes(search)) {
        continue
      }

      links.push(link)
    }
  }

  // Check if more available
  const hasMore = links.length > limit
  if (hasMore) {
    links.pop() // Remove extra item
  }

  return {
    links,
    cursor: result.cursor,
    hasMore
  }
}
```

### Index Design for Filtering

```typescript
// Multiple indexes for different queries
export async function createLinkIndexes(link: Link): Promise<void> {
  const promises = [
    // User index
    KV.put(`idx:user:${link.userId}:link:${link.slug}`, '1'),

    // Tag indexes
    ...link.tags.map(tag =>
      KV.put(`idx:tag:${tag}:link:${link.slug}`, '1')
    ),

    // Created_at index for time-based listing
    KV.put(
      `idx:time:${link.createdAt}:${link.slug}`,
      JSON.stringify({ slug: link.slug, userId: link.userId })
    )
  ]

  await Promise.all(promises)
}
```

---

## Chapter 4: Analytics Engine Storage

### Event Schema

```typescript
// shared/schemas/analytics.ts
export interface ClickEvent {
  // Blobs (strings) - for grouping
  blob1: string  // slug
  blob2: string  // country
  blob3: string  // city
  blob4: string  // continent
  blob5: string  // device
  blob6: string  // browser
  blob7: string  // os
  blob8: string  // referrer

  // Doubles (numbers) - for aggregation
  double1: number  // response_time_ms
  double2: number  // content_length

  // Indexes - for WHERE clauses
  indexes: string[]  // [slug, country, device]
}
```

### Writing Events

```typescript
// server/utils/analytics.ts
import { UAParser } from 'ua-parser-js'
import { getCountry } from '@cloudflare/geo-loc'

export async function trackClick(
  request: Request,
  slug: string,
  responseTime: number
): Promise<void> {
  const ua = new UAParser(request.headers.get('user-agent') || '')
  const geo = getCountry(request)

  const event: ClickEvent = {
    blob1: slug,
    blob2: geo?.country || 'unknown',
    blob3: geo?.city || 'unknown',
    blob4: geo?.continent || 'unknown',
    blob5: ua.getDevice().type || 'desktop',
    blob6: ua.getBrowser().name || 'unknown',
    blob7: ua.getOS().name || 'unknown',
    blob8: request.headers.get('referer') || 'direct',
    double1: responseTime,
    double2: 0,
    indexes: [slug, geo?.country || 'unknown']
  }

  ANALYTICS.writeDataPoint(event)
}
```

### Querying Analytics

```typescript
// server/api/analytics/[slug].get.ts
export interface AnalyticsResponse {
  totalClicks: number
  uniqueVisitors: number
  byCountry: Array<{ country: string; clicks: number }>
  byDevice: Array<{ device: string; clicks: number }>
  byBrowser: Array<{ browser: string; clicks: number }>
  byReferrer: Array<{ referrer: string; clicks: number }>
  timeSeries: Array<{ timestamp: string; clicks: number }>
}

export async function getAnalytics(
  slug: string,
  range: '24h' | '7d' | '30d' = '7d'
): Promise<AnalyticsResponse> {
  const interval = range === '24h' ? '1' : range === '7d' ? '7' : '30'

  // Total clicks
  const totalQuery = `
    SELECT SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${interval}' DAY
  `

  // By country
  const countryQuery = `
    SELECT blob2 as country, SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${interval}' DAY
    GROUP BY blob2
    ORDER BY clicks DESC
    LIMIT 20
  `

  // By device
  const deviceQuery = `
    SELECT blob5 as device, SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${interval}' DAY
    GROUP BY blob5
    ORDER BY clicks DESC
  `

  // Time series (hourly for 24h, daily for 7d/30d)
  const timeGranularity = range === '24h' ? 'hour' : 'day'
  const timeSeriesQuery = `
    SELECT
      DATE_TRUNC('${timeGranularity}', timestamp) as time,
      SUM(_sample_interval) as clicks
    FROM sink_analytics
    WHERE blob1 = '${slug}'
      AND timestamp > NOW() - INTERVAL '${interval}' DAY
    GROUP BY 1
    ORDER BY 1
  `

  const [total, country, device, browser, referrer, timeSeries] = await Promise.all([
    ANALYTICS.query(totalQuery),
    ANALYTICS.query(countryQuery),
    ANALYTICS.query(deviceQuery),
    ANALYTICS.query(`
      SELECT blob6 as browser, SUM(_sample_interval) as clicks
      FROM sink_analytics
      WHERE blob1 = '${slug}'
        AND timestamp > NOW() - INTERVAL '${interval}' DAY
      GROUP BY blob6
      ORDER BY clicks DESC
      LIMIT 10
    `),
    ANALYTICS.query(`
      SELECT blob8 as referrer, SUM(_sample_interval) as clicks
      FROM sink_analytics
      WHERE blob1 = '${slug}'
        AND timestamp > NOW() - INTERVAL '${interval}' DAY
      GROUP BY blob8
      ORDER BY clicks DESC
      LIMIT 10
    `),
    ANALYTICS.query(timeSeriesQuery)
  ])

  return {
    totalClicks: total[0]?.clicks || 0,
    uniqueVisitors: 0, // Requires additional HLL counting
    byCountry: country,
    byDevice: device,
    byBrowser: browser,
    byReferrer: referrer,
    timeSeries
  }
}
```

---

## Chapter 5: R2 Storage

### QR Code Storage

```typescript
// server/utils/qr-storage.ts
import QRCode from 'qrcode'

export async function generateQRCode(slug: string): Promise<string> {
  const key = `qr:${slug}.png`

  // Check cache
  const existing = await R2.get(key)
  if (existing) {
    return `https://sink.cool/qr/${slug}.png`
  }

  // Generate QR
  const url = `https://sink.cool/${slug}`
  const qrBuffer = await QRCode.toBuffer(url, {
    width: 512,
    margin: 2,
    errorCorrectionLevel: 'M'
  })

  // Store in R2
  await R2.put(key, qrBuffer, {
    httpMetadata: { contentType: 'image/png' }
  })

  return `https://sink.cool/qr/${slug}.png`
}

export async function deleteQRCode(slug: string): Promise<void> {
  await R2.delete(`qr:${slug}.png`)
}
```

### Image Upload Storage

```typescript
// server/api/upload.post.ts
import { nanoid } from 'nanoid'

export default eventHandler(async (event) => {
  const formData = await readFormData(event)
  const file = formData.get('image') as File

  if (!file) {
    throw createError({ status: 400, message: 'No file uploaded' })
  }

  // Validate file type
  const allowedTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp']
  if (!allowedTypes.includes(file.type)) {
    throw createError({ status: 400, message: 'Invalid file type' })
  }

  // Validate file size (max 5MB)
  if (file.size > 5 * 1024 * 1024) {
    throw createError({ status: 400, message: 'File too large (max 5MB)' })
  }

  // Generate unique key
  const ext = file.name.split('.').pop()
  const key = `uploads/${nanoid()}.${ext}`

  // Store in R2
  await R2.put(key, file.stream(), {
    httpMetadata: { contentType: file.type }
  })

  return {
    url: `https://sink.cool/${key}`
  }
})
```

---

## Chapter 6: Caching Strategies

### Edge Caching with Cache API

```typescript
// server/utils/cache.ts
export async function cachedFetch<T>(
  key: string,
  fetchFn: () => Promise<T>,
  ttl: number = 3600
): Promise<T> {
  const cacheKey = `cache:${key}`

  // Try cache first
  const cached = await KV.get(cacheKey)
  if (cached) {
    const { value, expiresAt } = JSON.parse(cached)
    if (expiresAt > Date.now()) {
      return value as T
    }
  }

  // Fetch fresh
  const value = await fetchFn()

  // Update cache
  await KV.put(cacheKey, JSON.stringify({
    value,
    expiresAt: Date.now() + (ttl * 1000)
  }), {
    expirationTtl: ttl
  })

  return value
}

// Usage in API
export default eventHandler(async (event) => {
  const { slug } = event.context.params
  const analytics = await cachedFetch(
    `analytics:${slug}`,
    () => getAnalytics(slug),
    300 // 5 minute cache
  )
  return analytics
})
```

### Cache Invalidation

```typescript
// Invalidate cache on update
export async function invalidateLinkCache(slug: string): Promise<void> {
  const cacheKeys = [
    `cache:link:${slug}`,
    `cache:analytics:${slug}`,
    `cache:user:${slug}:links`
  ]

  await Promise.all(
    cacheKeys.map(key => KV.delete(key))
  )
}
```

---

## Chapter 7: Data Migration

### Export/Import Links

```typescript
// server/api/export.get.ts
export default eventHandler(async (event) => {
  const { userId } = event.context.auth

  const links: Link[] = []
  let cursor: string | undefined

  do {
    const result = await KV.list({
      prefix: `idx:user:${userId}:link:`,
      cursor
    })

    for (const key of result.keys) {
      const linkData = await KV.get(`link:${key.name.split(':').pop()}`)
      if (linkData) {
        links.push(JSON.parse(linkData))
      }
    }

    cursor = result.cursor
  } while (cursor)

  return links
})

// server/api/import.post.ts
export default eventHandler(async (event) => {
  const { userId } = event.context.auth
  const links = await readBody(event) as Link[]

  const results = { success: 0, failed: 0, errors: [] }

  for (const link of links) {
    try {
      // Validate
      await LinkSchema.parseAsync(link)

      // Check ownership
      const existing = await getLink(link.slug)
      if (existing && existing.userId !== userId) {
        results.failed++
        results.errors.push(`Slug "${link.slug}" exists`)
        continue
      }

      // Import
      await createLink({ ...link, userId })
      results.success++
    } catch (error) {
      results.failed++
      results.errors.push(error.message)
    }
  }

  return results
})
```

---

## Summary

Sink's storage architecture:

1. **Workers KV** - Primary storage with namespaced keys
2. **Analytics Engine** - Click events with blob/double fields
3. **R2** - QR codes and image uploads
4. **Caching** - Edge cache with TTL-based invalidation
5. **Indexes** - Separate keys for efficient listing

---

## Next Steps

See [rust-revision.md](./rust-revision.md) for implementing this storage architecture in Rust with valtron.
