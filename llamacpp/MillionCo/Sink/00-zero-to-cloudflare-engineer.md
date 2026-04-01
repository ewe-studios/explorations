---
title: "Zero to Cloudflare Engineer"
subtitle: "Understanding serverless edge computing with Cloudflare Workers"
---

# Zero to Cloudflare Engineer

## Introduction

This guide takes you from zero knowledge to understanding how to build applications like Sink on Cloudflare's serverless platform.

---

## Chapter 1: What is Cloudflare Workers?

### The Problem with Traditional Hosting

Traditional web apps run on servers:
```
User -> Load Balancer -> Web Server -> Database
       (AWS/Azure)       (Your Code)   (MySQL/Postgres)
```

**Problems:**
- Cold starts (server boot time)
- Scaling requires provisioning
- Geographic latency (users far from server)
- Pay for idle capacity

### The Cloudflare Solution

Cloudflare Workers run your code at the **edge** - in 275+ cities worldwide:
```
User -> Cloudflare Edge (1ms away) -> Workers KV (edge storage)
       (Your code runs here)
```

**Benefits:**
- No cold starts (pre-initialized)
- Auto-scales to millions of requests
- Users get low latency (code runs near them)
- Pay per request (not per server)

### What Makes Workers Different?

| Feature | AWS Lambda | Cloudflare Workers |
|---------|------------|-------------------|
| Runtime | Node.js/Python/etc | V8 Isolates |
| Cold Start | 100ms-5s | ~0ms |
| Memory Limit | Up to 10GB | 128MB |
| Execution Time | Up to 15min | Up to 30s |
| Pricing | Per 100ms | Per 1ms |

**V8 Isolates** are Chrome's JavaScript runtime, but without the browser overhead. They're:
- Lightweight (~1ms startup)
- Secure (sandboxed)
- Fast (native machine code)

---

## Chapter 2: Workers KV (Key-Value Storage)

### What is Workers KV?

Workers KV is Cloudflare's edge key-value store. Think of it as Redis, but global.

```
┌─────────────────────────────────────────────┐
│            Cloudflare Network               │
│  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐        │
│  │ NYC │  │ LON │  │ SIN │  │ SYD │  ...   │
│  │ KV  │  │ KV  │  │ KV  │  │ KV  │        │
│  └─────┘  └─────┘  └─────┘  └─────┘        │
│     │        │        │        │            │
│     └────────┴────────┴────────┘            │
│                  │                          │
│           Replicated Storage                │
└─────────────────────────────────────────────┘
```

### Key Concepts

**Eventual Consistency:**
- Writes propagate globally (~60s)
- Reads are immediate (from local edge)
- Good for: configs, user profiles, links
- Not for: financial transactions, counters

**Keys and Values:**
```typescript
// Write
await KV.put('link:abc123', JSON.stringify({
  url: 'https://example.com',
  createdAt: Date.now()
}))

// Read
const data = await KV.get('link:abc123')
const link = JSON.parse(data)

// Delete
await KV.delete('link:abc123')

// List
const keys = await KV.list({ prefix: 'link:' })
```

### Data Modeling for Sink

```typescript
// Link storage pattern
interface LinkRecord {
  id: string      // nanoid(10)
  slug: string    // "abc123" or custom
  url: string     // Target URL
  createdAt: number
  expiresAt?: number
}

// Key structure
`link:${slug}` => LinkRecord

// Index pattern (for listing)
`user:${userId}:links:${slug}` => slug

// Metadata pattern
`link:${slug}:meta` => { clicks: number, lastClick: timestamp }
```

---

## Chapter 3: Analytics Engine

### What is Analytics Engine?

Analytics Engine is Cloudflare's time-series database for edge events.

**Use Cases:**
- Click tracking
- Page views
- API call logging
- Performance metrics

### Writing Events

```typescript
// In your Worker
const event = {
  blob1: slug,           // Link slug
  blob2: country,        // User's country
  blob3: device,         // Device type
  blob4: browser,        // Browser name
  blob5: referrer,       // Referrer URL
  double1: responseTime, // Response time in ms
  indexes: [slug]        // Index for querying
}

ANALYTICS.writeDataPoint(event)
```

### Querying Events

```sql
SELECT
  blob1 as slug,
  SUM(_sample_interval) as clicks,
  COUNT(*) as impressions
FROM sink_analytics
WHERE timestamp > NOW() - INTERVAL '24' HOUR
GROUP BY blob1
ORDER BY clicks DESC
LIMIT 100
```

### Why Analytics Engine?

| Feature | ClickHouse | Analytics Engine |
|---------|------------|------------------|
| Query Speed | Fast | Fast |
| Setup | Self-hosted | Managed |
| Location | Your infrastructure | Edge |
| Cost | Infrastructure cost | Pay per query |

---

## Chapter 4: Workers AI

### What is Workers AI?

Workers AI lets you run AI models at the edge without GPU infrastructure.

**Available Models:**
- `@cf/meta/llama-3-8b` - Text generation
- `@cf/microsoft/resnet-50` - Image classification
- `@cf/baai/bge-base-en-v1.5` - Text embeddings
- `@cf/mistral/mistral-7b-instruct` - Chat

### Using AI in Sink

```typescript
// Generate a slug from URL
const response = await AI.run('@cf/meta/llama-3-8b', {
  prompt: `Generate a 6-character alphanumeric slug for this URL: ${url}. Only return the slug, nothing else.`
})

const slug = response.response.trim().toLowerCase()
```

### Cost Comparison

| Provider | Cost per 1M tokens |
|----------|-------------------|
| OpenAI GPT-4 | $30 |
| Anthropic Claude | $15 |
| Workers AI (Llama 3) | $3 |

---

## Chapter 5: R2 Storage

### What is R2?

R2 is Cloudflare's S3-compatible object storage.

**Use Cases:**
- Image uploads
- Video storage
- Static assets
- Backup archives

### Why R2 over S3?

| Feature | AWS S3 | Cloudflare R2 |
|---------|--------|---------------|
| Storage | $0.023/GB | $0.015/GB |
| **Egress** | $0.09/GB | **$0** |
| PUT requests | $0.005/1K | $0.005/1K |
| GET requests | $0.0004/1K | $0.0004/1K |

**No egress fees** is the killer feature - you don't pay when users download files.

### Using R2 in Sink

```typescript
// Upload image
const image = await req.formData()
const file = image.get('image')

await R2.put(`uploads/${slug}`, file.stream(), {
  httpMetadata: { contentType: file.type }
})

// Get image
const object = await R2.get(`uploads/${slug}`)
if (!object) return new Response('Not found', { status: 404 })

return new Response(object.body, {
  headers: { 'Content-Type': object.httpContentType }
})
```

---

## Chapter 6: Pages + Workers Integration

### What is Cloudflare Pages?

Pages is Cloudflare's Jamstack hosting (like Vercel or Netlify).

**Features:**
- Git-based deployments
- Preview deployments
- Automatic HTTPS
- Built-in analytics

### Pages + Workers Architecture

```
┌─────────────────────────────────────┐
│         Cloudflare Pages            │
│  ┌─────────────────────────────┐    │
│  │   Nuxt Frontend (static)    │    │
│  │   HTML, CSS, JS, Images     │    │
│  └─────────────────────────────┘    │
│                  │                  │
│  ┌───────────────▼───────────────┐  │
│  │   Pages Functions (Worker)    │  │
│  │   /api/* routes               │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

### File-Based Routing

```
pages/
├── index.vue              # /
├── dashboard/
│   ├── index.vue          # /dashboard
│   └── links/
│       └── [slug].vue     # /dashboard/links/:slug
└── [slug].vue             # /:slug (catch-all redirect)
```

### API Routes

```
server/api/
├── link/
│   ├── create.post.ts     # POST /api/link
│   ├── list.get.ts        # GET /api/link
│   └── [slug].delete.ts   # DELETE /api/link/:slug
├── analytics/
│   └── [slug].get.ts      # GET /api/analytics/:slug
└── ai/
    └── slug.post.ts       # POST /api/ai/slug
```

**Naming Convention:**
- `.get.ts` - GET request
- `.post.ts` - POST request
- `.put.ts` - PUT request
- `.delete.ts` - DELETE request
- `.patch.ts` - PATCH request

---

## Chapter 7: Nuxt 4 Fundamentals

### What is Nuxt?

Nuxt is a Vue.js framework for building full-stack applications.

**Key Features:**
- File-based routing
- Auto-imports
- Server routes
- SSR/SSG support
- Module ecosystem

### Nuxt 4 New Features

1. **Hybrid Rendering** - Mix SSR and CSR per-component
2. **Better TypeScript** - Infer types from server to client
3. **Cloudflare Native** - Built-in Workers adapter
4. **Faster Builds** - Incremental builds with Rolldown

### Directory Structure

```
app/
├── components/      # Vue components (auto-imported)
├── composables/     # Vue composables (auto-imported)
├── pages/           # File-based routing
├── layouts/         # Page layouts
├── plugins/         # Runtime plugins
├── utils/           # Utility functions
├── types/           # TypeScript types
└── lib/             # Shared helpers
```

### Auto-Imports

Nuxt auto-imports:
- Vue APIs: `ref`, `computed`, `watch`
- Nuxt APIs: `useFetch`, `useState`, `useRuntimeConfig`
- Components: Any `.vue` in `components/`
- Utils: Functions in `utils/`

```typescript
// No import needed!
const count = ref(0)
const { data } = await useFetch('/api/links')
```

---

## Chapter 8: Zod Schema Validation

### What is Zod?

Zod is a TypeScript-first schema validation library.

```typescript
import { z } from 'zod'

// Define schema
const LinkSchema = z.object({
  id: z.string().min(1),
  slug: z.string().regex(/^[a-z0-9]+$/),
  url: z.string().url(),
  createdAt: z.number()
})

// Validate at runtime
const result = LinkSchema.safeParse(data)
if (!result.success) {
  throw new Error(result.error.message)
}

// Infer TypeScript type
type Link = z.infer<typeof LinkSchema>
```

### Why Zod?

1. **Single Source of Truth** - Schema = Type
2. **Runtime Validation** - Catch invalid data
3. **Error Messages** - Clear validation errors
4. **Composition** - Build complex schemas

---

## Chapter 9: shadcn-vue Components

### What is shadcn-vue?

shadcn-vue is a component library built with Radix UI primitives and TailwindCSS.

**Unlike other libraries:**
- Components are **your code** (copy-paste, not npm)
- Full control over styling
- Accessible by default
- No runtime dependency

### Using shadcn-vue

```vue
<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
</script>

<template>
  <Button variant="primary">
    Click me
  </Button>
</template>
```

### Component Structure

```
components/ui/button/
├── Button.vue       # Main component
├── ButtonVariants   # Variant definitions
└── index.ts         # Re-exports
```

---

## Chapter 10: Testing with Vitest

### What is Vitest?

Vitest is a Vite-native test framework (like Jest but faster).

```typescript
// tests/example.test.ts
import { describe, it, expect, beforeEach } from 'vitest'

describe('Link API', () => {
  beforeEach(async () => {
    // Reset state before each test
    await KV.clear()
  })

  it('creates a link', async () => {
    const response = await fetch('/api/link', {
      method: 'POST',
      body: JSON.stringify({ url: 'https://example.com' })
    })

    expect(response.status).toBe(200)
    const data = await response.json()
    expect(data.slug).toBeDefined()
  })
})
```

### Cloudflare Workers Testing

```typescript
// vitest.config.ts
import { defineWorkersConfig } from '@cloudflare/vitest-pool-workers/config'

export default defineWorkersConfig({
  test: {
    poolOptions: {
      workers: {
        wrangler: { configPath: './wrangler.jsonc' }
      }
    }
  }
})
```

### Testing Patterns

```typescript
// Mock external services
vi.mock('./ai', () => ({
  generateSlug: vi.fn(() => 'abc123')
}))

// Test with real KV
import { SELF } from 'cloudflare:test'

it('fetches link', async () => {
  // Create test data
  await KV.put('link:test', JSON.stringify({ url: 'https://example.com' }))

  const response = await SELF.fetch('/api/link/test')
  expect(response.status).toBe(200)
})
```

---

## Summary

You now understand:

1. **Cloudflare Workers** - Edge compute with V8 isolates
2. **Workers KV** - Global key-value storage
3. **Analytics Engine** - Time-series event storage
4. **Workers AI** - Edge AI inference
5. **R2 Storage** - S3-compatible object storage
6. **Pages + Workers** - Full-stack architecture
7. **Nuxt 4** - Vue framework for full-stack apps
8. **Zod** - Schema validation
9. **shadcn-vue** - Component library
10. **Vitest** - Testing framework

---

## Next Steps

To go deeper:
1. Deploy a simple Worker
2. Set up Workers KV
3. Build a Nuxt app on Pages
4. Add Analytics Engine tracking
5. Integrate Workers AI
