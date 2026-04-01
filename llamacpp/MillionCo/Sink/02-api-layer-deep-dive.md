---
title: "API Layer Deep Dive"
subtitle: "File-based routing, Zod validation, and error handling patterns"
---

# API Layer Deep Dive

## Overview

Sink's API layer is built on Nuxt's Nitro server with Cloudflare Workers runtime. It uses:
- **File-based routing** - Auto-generated routes from file structure
- **Zod validation** - Type-safe request validation
- **Middleware composition** - Reusable auth and rate limiting
- **Structured error handling** - Consistent error responses

---

## Chapter 1: File-Based Routing

### How Nitro Routing Works

Nitro scans the `server/api/` directory and creates routes based on file names:

```
server/api/
├── link/
│   ├── create.post.ts      → POST /api/link/create
│   ├── list.get.ts         → GET /api/link/list
│   └── [slug].delete.ts    → DELETE /api/link/:slug
├── analytics/
│   └── [slug].get.ts       → GET /api/analytics/:slug
└── health.get.ts           → GET /api/health
```

### Route Parameters

```typescript
// server/api/link/[slug].get.ts
export default eventHandler(async (event) => {
  // Dynamic params from [slug]
  const { slug } = event.context.params

  // Query params from URL
  const query = getQuery(event)
  const { includeAnalytics } = query

  // ... handler logic
})
```

### Method Suffixes

| Suffix | HTTP Method | Example |
|--------|-------------|---------|
| `.get.ts` | GET | `list.get.ts` → GET |
| `.post.ts` | POST | `create.post.ts` → POST |
| `.put.ts` | PUT | `update.put.ts` → PUT |
| `.patch.ts` | PATCH | `update.patch.ts` → PATCH |
| `.delete.ts` | DELETE | `delete.delete.ts` → DELETE |
| (no suffix) | All | `handler.ts` → ALL methods |

### Catch-All Routes

```typescript
// server/api/link/[...slug].ts
// Matches: /api/link/abc, /api/link/abc/def, etc.

export default eventHandler(async (event) => {
  const { slug } = event.context.params
  // slug = "abc" or "abc/def"
})
```

---

## Chapter 2: Request Validation with Zod

### Shared Schema Definition

```typescript
// shared/schemas/link.ts
import { z } from 'zod'

export const CreateLinkSchema = z.object({
  url: z
    .string()
    .trim()
    .url('Must be a valid URL')
    .max(2048, 'URL too long (max 2048 characters)'),

  slug: z
    .string()
    .trim()
    .max(100, 'Slug too long')
    .regex(
      /^[a-zA-Z0-9]+(-[a-zA-Z0-9]+)*$/,
      'Slug can only contain letters, numbers, and hyphens'
    )
    .optional(),

  expiresAt: z
    .number()
    .int()
    .positive()
    .optional(),

  password: z
    .string()
    .max(100)
    .optional(),

  og: z
    .object({
      title: z.string().max(512),
      description: z.string().max(1024),
      image: z.string().url().optional()
    })
    .optional(),

  deviceRouting: z
    .object({
      ios: z.string().url().optional(),
      android: z.string().url().optional()
    })
    .optional()
})

export type CreateLinkInput = z.infer<typeof CreateLinkSchema>
```

### Validation Middleware

```typescript
// server/utils/validate.ts
import type { ZodSchema } from 'zod'
import { readBody } from 'h3'

export function validateBody<T>(schema: ZodSchema<T>) {
  return async (event: H3Event): Promise<T> => {
    const body = await readBody(event)
    const result = await schema.safeParseAsync(body)

    if (!result.success) {
      throw createError({
        status: 400,
        message: 'Validation failed',
        data: {
          errors: result.error.errors.map(err => ({
            field: err.path.join('.'),
            message: err.message
          }))
        }
      })
    }

    return result.data
  }
}

// Usage in API route
// server/api/link/create.post.ts
import { CreateLinkSchema } from '~/shared/schemas/link'
import { validateBody } from '~/server/utils/validate'

export default eventHandler(async (event) => {
  const input = await validateBody(CreateLinkSchema)(event)

  // input is fully typed!
  const { url, slug, expiresAt } = input

  // ... create link logic
})
```

### Query Parameter Validation

```typescript
// server/utils/validate-query.ts
export function validateQuery<T>(schema: ZodSchema<T>) {
  return (event: H3Event): T => {
    const query = getQuery(event)
    const result = schema.safeParse(query)

    if (!result.success) {
      throw createError({
        status: 400,
        message: 'Invalid query parameters',
        data: result.error.flatten()
      })
    }

    return result.data
  }
}

// Usage
// server/api/link/list.get.ts
const ListLinksSchema = z.object({
  limit: z.string().transform(Number).int().min(1).max(100).default('20'),
  cursor: z.string().optional(),
  search: z.string().optional(),
  sortBy: z.enum(['createdAt', 'slug', 'clicks']).default('createdAt'),
  sortOrder: z.enum(['asc', 'desc']).default('desc')
})

export default eventHandler(async (event) => {
  const query = validateQuery(ListLinksSchema)(event)
  const { limit, cursor, search, sortBy, sortOrder } = query

  // ... list links logic
})
```

---

## Chapter 3: Error Handling Patterns

### Creating Errors

```typescript
// server/utils/errors.ts
import { createError, type H3Error } from 'h3'

interface ApiErrorOptions {
  status: number
  message: string
  code?: string
  data?: Record<string, unknown>
}

export function apiError(options: ApiErrorOptions): H3Error {
  return createError({
    status: options.status,
    message: options.message,
    code: options.code || 'API_ERROR',
    data: options.data
  })
}

// Convenience functions
export const notFound = (resource: string) =>
  apiError({ status: 404, message: `${resource} not found` })

export const conflict = (message: string) =>
  apiError({ status: 409, message })

export const unauthorized = (message = 'Unauthorized') =>
  apiError({ status: 401, message })

export const forbidden = (message = 'Forbidden') =>
  apiError({ status: 403, message })

export const badRequest = (message: string) =>
  apiError({ status: 400, message })
```

### Error Response Format

```typescript
// server/plugins/error-handler.ts
export default defineNitroPlugin((nitroApp) => {
  nitroApp.hooks.hook('error', (error, { event }) => {
    // Log error
    console.error('[API Error]', {
      path: event.path,
      method: event.method,
      error: error.message,
      stack: error.stack
    })

    // Format response
    if (error.code === 'API_ERROR') {
      return {
        success: false,
        error: {
          code: error.code,
          message: error.message,
          details: error.data
        }
      }
    }

    // Generic error
    return {
      success: false,
      error: {
        code: 'INTERNAL_ERROR',
        message: 'An unexpected error occurred'
      }
    }
  })
})
```

### Try-Catch Pattern

```typescript
// server/api/link/[slug].delete.ts
export default eventHandler(async (event) => {
  try {
    const { slug } = event.context.params
    const { userId } = event.context.auth

    // Get link
    const link = await getLink(slug)
    if (!link) {
      throw notFound('Link')
    }

    // Check ownership
    if (link.userId && link.userId !== userId) {
      throw forbidden('You can only delete your own links')
    }

    // Delete
    await deleteLink(slug)

    return { success: true }
  } catch (error) {
    // Let Nitro handle formatting
    throw error
  }
})
```

---

## Chapter 4: Authentication Middleware

### Auth Composable

```typescript
// server/utils/auth.ts
import { getHeader, createError } from 'h3'
import { verify } from 'h3-jwt'

interface JWTPayload {
  sub: string  // User ID
  email: string
}

export interface AuthContext {
  userId: string
  email: string
  isAdmin: boolean
}

export async function getAuth(event: H3Event): Promise<AuthContext | null> {
  const authHeader = getHeader(event, 'Authorization')

  if (!authHeader?.startsWith('Bearer ')) {
    return null
  }

  const token = authHeader.slice(7)

  try {
    const payload = await verify<JWTPayload>(
      token,
      process.env.JWT_SECRET!
    )

    return {
      userId: payload.sub,
      email: payload.email,
      isAdmin: payload.email.endsWith('@admin.com')
    }
  } catch {
    return null
  }
}

export async function requireAuth(event: H3Event): Promise<AuthContext> {
  const auth = await getAuth(event)

  if (!auth) {
    throw createError({
      status: 401,
      message: 'Authentication required'
    })
  }

  return auth
}

export async function requireAdmin(event: H3Event): Promise<AuthContext> {
  const auth = await requireAuth(event)

  if (!auth.isAdmin) {
    throw createError({
      status: 403,
      message: 'Admin access required'
    })
  }

  return auth
}
```

### Using Auth in Routes

```typescript
// server/api/link/create.post.ts
export default eventHandler(async (event) => {
  // Optional auth - works for both authenticated and anonymous
  const auth = await getAuth(event)

  const input = await validateBody(CreateLinkSchema)(event)

  // Generate slug if not provided
  const slug = input.slug || await generateSlug(input.url)

  const link: Link = {
    ...input,
    slug,
    userId: auth?.userId,  // Associate with user if logged in
    createdAt: Date.now()
  }

  await createLink(link)

  return { success: true, link }
})

// server/api/user/links.get.ts - requires auth
export default eventHandler(async (event) => {
  const auth = await requireAuth(event)

  const links = await listLinks({ userId: auth.userId })

  return { links }
})
```

---

## Chapter 5: Rate Limiting

### Token Bucket Implementation

```typescript
// server/utils/rate-limit.ts
interface RateLimitState {
  tokens: number
  lastRefill: number
}

async function checkRateLimit(
  identifier: string,
  limit: number,
  windowMs: number
): Promise<{ allowed: boolean; remaining: number; reset: number }> {
  const key = `ratelimit:${identifier}`
  const now = Date.now()

  // Get current state
  const stateJson = await KV.get(key)
  let state: RateLimitState = stateJson
    ? JSON.parse(stateJson)
    : { tokens: limit, lastRefill: now }

  // Refill tokens based on time elapsed
  const elapsed = now - state.lastRefill
  const refill = Math.floor((elapsed / windowMs) * limit)
  state.tokens = Math.min(limit, state.tokens + refill)
  state.lastRefill = now

  // Check if allowed
  const allowed = state.tokens > 0

  if (allowed) {
    state.tokens--
  }

  // Save state
  await KV.put(key, JSON.stringify(state), {
    expirationTtl: Math.ceil(windowMs / 1000)
  })

  return {
    allowed,
    remaining: state.tokens,
    reset: now + windowMs
  }
}

export function rateLimit(options: {
  limit: number
  windowMs: number
  key?: (event: H3Event) => string
}) {
  return async (event: H3Event) => {
    const identifier = options.key
      ? options.key(event)
      : `ip:${getRequestIP(event)}`

    const result = await checkRateLimit(
      identifier,
      options.limit,
      options.windowMs
    )

    // Set rate limit headers
    setHeader(event, 'X-RateLimit-Limit', options.limit.toString())
    setHeader(event, 'X-RateLimit-Remaining', result.remaining.toString())
    setHeader(event, 'X-RateLimit-Reset', result.reset.toString())

    if (!result.allowed) {
      throw createError({
        status: 429,
        message: 'Too many requests',
        data: {
          retryAfter: Math.ceil((result.reset - Date.now()) / 1000)
        }
      })
    }
  }
}
```

### Usage in Routes

```typescript
// server/api/link/create.post.ts
const rateLimiter = rateLimit({
  limit: 10,      // 10 requests
  windowMs: 60000 // per minute
})

export default eventHandler(async (event) => {
  // Apply rate limiting
  await rateLimiter(event)

  // ... rest of handler
})
```

---

## Chapter 6: CORS Configuration

### Global CORS Setup

```typescript
// server/middleware/cors.ts
export default defineEventHandler((event) => {
  // Handle preflight
  if (event.method === 'OPTIONS') {
    setHeader(event, 'Access-Control-Allow-Origin', '*')
    setHeader(event, 'Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, PATCH, OPTIONS')
    setHeader(event, 'Access-Control-Allow-Headers', 'Content-Type, Authorization')
    setHeader(event, 'Access-Control-Max-Age', '86400')
    return null
  }

  // Set CORS headers for all responses
  setHeader(event, 'Access-Control-Allow-Origin', '*')
  setHeader(event, 'Access-Control-Allow-Credentials', 'true')
  setHeader(event, 'Access-Control-Expose-Headers', 'X-RateLimit-Limit, X-RateLimit-Remaining')
})
```

---

## Chapter 7: Response Formatting

### Standard Response Format

```typescript
// server/utils/response.ts
export function successResponse<T>(data: T, meta?: Record<string, unknown>) {
  return {
    success: true,
    data,
    meta,
    timestamp: Date.now()
  }
}

export function paginatedResponse<T>(
  items: T[],
  options: {
    cursor?: string
    hasMore: boolean
    total?: number
  }
) {
  return successResponse(items, {
    pagination: {
      cursor: options.cursor,
      hasMore: options.hasMore,
      total: options.total
    }
  })
}

// Usage in API route
// server/api/link/list.get.ts
export default eventHandler(async (event) => {
  const { links, cursor, hasMore } = await listLinks({})

  return paginatedResponse(links, { cursor, hasMore })
})
```

---

## Summary

Sink's API layer demonstrates:

1. **File-based routing** - Convention over configuration
2. **Zod validation** - Type-safe request handling
3. **Structured errors** - Consistent error responses
4. **Auth middleware** - Reusable authentication
5. **Rate limiting** - Token bucket algorithm with KV storage
6. **CORS** - Global middleware setup
7. **Response formatting** - Standard API responses

---

## Next Steps

See [rust-revision.md](./rust-revision.md) for implementing this API layer in Rust with valtron.
