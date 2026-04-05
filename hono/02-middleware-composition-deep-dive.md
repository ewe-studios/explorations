---
source: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/hono
repository: github.com/honojs/hono
explored_at: 2026-04-05
focus: Middleware composition, Koa-style async chains, context propagation, error handling, execution flow
---

# Deep Dive: Middleware Composition System

## Overview

This deep dive examines Hono's middleware composition system - the core pattern that enables Koa-style async middleware chains, context propagation between handlers, and flexible request/response processing.

## Architecture

```mermaid
flowchart TB
    Request[HTTP Request] --> Adapter[Runtime Adapter]
    Adapter --> Compose[Compose Function]
    
    subgraph Composition["Middleware Chain"]
        MW1[Global Middleware 1]
        MW2[Global Middleware 2]
        MW3[Route Middleware]
        Handler[Route Handler]
    end
    
    Compose --> MW1
    MW1 -->|next()| MW2
    MW2 -->|next()| MW3
    MW3 -->|next()| Handler
    
    Handler --> Response[Response]
    Response --> MW3
    MW3 --> MW2
    MW2 --> MW1
    MW1 --> Client[Client]
    
    subgraph Context["Context Object"]
        C1[Request Data]
        C2[Custom Values]
        C3[Response State]
    end
    
    MW1 -.set/get.-> C1
    MW2 -.set/get.-> C2
    MW3 -.set/get.-> C3
```

## Compose Function

```typescript
// src/compose.ts - Core middleware composition

import type { Context } from './context'
import type { Next } from './types'

type Middleware<T> = (c: T, next: Next) => Promise<void>

/**
 * Compose middleware into a single handler
 * 
 * This is the heart of Hono's middleware system.
 * It creates a Promise chain where each middleware
 * can run code before AND after the next middleware.
 */
export const compose = <T extends Context>(
  middleware: Middleware<T>[],
  onError?: (err: Error, c: T) => Response | Promise<Response>,
  onNotFound?: (c: T) => Response | Promise<Response>
) => {
  return (c: T, handler: (c: T) => Response | Promise<Response>) => {
    const index = 0
    
    // Recursive next function
    const next = async (currentIndex: number = index) => {
      // If we've passed all middleware, call handler
      if (currentIndex >= middleware.length) {
        return handler(c)
      }
      
      const middlewareFn = middleware[currentIndex]
      
      try {
        // Call middleware with context and next function
        await middlewareFn(c, async () => {
          // next() calls the next middleware
          return await next(currentIndex + 1)
        })
      } catch (err) {
        // Error handling
        if (onError) {
          return onError(err as Error, c)
        }
        throw err
      }
    }
    
    return next()
  }
}

export type Compose = typeof compose
```

## How Composition Works

```typescript
// Example: Understanding the composition flow

// Three middleware that log before/after
const mw1 = async (c: Context, next: Next) => {
  console.log('MW1: Before')
  await next()
  console.log('MW1: After')
}

const mw2 = async (c: Context, next: Next) => {
  console.log('MW2: Before')
  await next()
  console.log('MW2: After')
}

const mw3 = async (c: Context, next: Next) => {
  console.log('MW3: Before')
  await next()
  console.log('MW3: After')
}

// Handler
const handler = (c: Context) => {
  console.log('Handler: Executing')
  return c.text('Hello')
}

// Composed execution:
const composed = compose([mw1, mw2, mw3])

// Execution order when called:
// MW1: Before
//   MW2: Before
//     MW3: Before
//       Handler: Executing
//     MW3: After
//   MW2: After
// MW1: After

// This is possible because each middleware awaits next()
// The "After" code runs after the Promise from next() resolves
```

## Context Object

```typescript
// src/context.ts - The Context class

export class Context {
  // Request
  req: Request
  private rawRequest: Request
  
  // Response state
  private status: number = 200
  private headers: Headers
  private body?: BodyInit
  
  // Storage for middleware communication
  private #variables: Map<string, any> = new Map()
  
  // ExecutionContext (for background tasks)
  executionCtx?: ExecutionContext
  
  constructor(req: Request, options: ContextOptions) {
    this.rawRequest = req
    this.req = new HonoRequest(req)
    this.headers = new Headers(options.headers)
  }
  
  // === Request Access ===
  
  get url(): string {
    return this.req.url
  }
  
  get method(): string {
    return this.req.method
  }
  
  get path(): string {
    return this.req.path
  }
  
  header(name: string): string | null
  header(name: string, value: string): void
  header(name: string, value?: string) {
    if (value === undefined) {
      return this.req.headers.get(name)
    }
    this.headers.set(name, value)
  }
  
  // === Response Helpers ===
  
  text(text: string, status?: number, headers?: Headers): Response {
    this.status = status ?? this.status
    if (headers) this.headers = mergeHeaders(this.headers, headers)
    return new Response(text, {
      status: this.status,
      headers: {
        'Content-Type': 'text/plain; charset=UTF-8',
        ...this.headers,
      },
    })
  }
  
  json<T>(object: T, status?: number, headers?: Headers): Response {
    this.status = status ?? this.status
    if (headers) this.headers = mergeHeaders(this.headers, headers)
    return new Response(JSON.stringify(object), {
      status: this.status,
      headers: {
        'Content-Type': 'application/json',
        ...this.headers,
      },
    })
  }
  
  html(html: string, status?: number, headers?: Headers): Response {
    this.status = status ?? this.status
    if (headers) this.headers = mergeHeaders(this.headers, headers)
    return new Response(html, {
      status: this.status,
      headers: {
        'Content-Type': 'text/html; charset=UTF-8',
        ...this.headers,
      },
    })
  }
  
  redirect(location: string, status: number = 302): Response {
    return new Response(null, {
      status,
      headers: { Location: location },
    })
  }
  
  // === Context Storage ===
  
  set<Key extends keyof Variables>(key: Key, value: Variables[Key]): void {
    this.#variables.set(key as string, value)
  }
  
  get<Key extends keyof Variables>(key: Key): Variables[Key] | undefined {
    return this.#variables.get(key as string)
  }
  
  // === Rendering ===
  
  render(component: FC<Props>, props: Props): Response {
    const html = renderToString(component(props))
    return this.html(html)
  }
  
  // === Build Response ===
  
  newResponse(data: BodyInit, status?: number, headers?: Headers): Response {
    return new Response(data, {
      status: status ?? this.status,
      headers: headers ?? this.headers,
    })
  }
}
```

## Context Variables Pattern

```typescript
// Type-safe context variables

interface AppVariables {
  user: { id: number; role: string }
  db: Database
  requestId: string
}

// Extend Context type
type AppContext = Context<{ Variables: AppVariables }>

// Auth middleware sets user
const auth = async (c: AppContext, next: Next) => {
  const token = c.req.header('Authorization')
  
  if (token) {
    const user = await validateToken(token)
    c.set('user', user)  // Type-safe
  }
  
  await next()
}

// DB middleware sets database connection
const withDb = async (c: AppContext, next: Next) => {
  const db = new Database()
  c.set('db', db)
  
  await next()
  
  // Cleanup after request
  await db.close()
}

// Handler can access typed values
const handler = async (c: AppContext) => {
  const user = c.get('user')  // Type: { id: number; role: string } | undefined
  const db = c.get('db')      // Type: Database | undefined
  
  if (!user) {
    return c.json({ error: 'Unauthorized' }, 401)
  }
  
  const posts = await db.query('SELECT * FROM posts WHERE user_id = ?', [user.id])
  return c.json({ posts })
}
```

## Built-in Middleware Internals

### Logger Middleware

```typescript
// src/middleware/logger/index.ts

export const logger = () => {
  return async (c: Context, next: Next) => {
    const start = Date.now()
    
    // Log request
    console.log(`${c.req.method} ${c.req.url}`)
    
    await next()
    
    // Log response after handler completes
    const duration = Date.now() - start
    console.log(
      `${c.res.status} ${c.req.method} ${c.req.path} - ${duration}ms`
    )
  }
}

// Usage:
app.use('*', logger())
// Output:
// GET http://localhost:3000/users
// 200 GET /users - 15ms
```

### CORS Middleware

```typescript
// src/middleware/cors/index.ts

interface CorsOptions {
  origin?: string | string[]
  allowMethods?: string[]
  allowHeaders?: string[]
  maxAge?: number
  credentials?: boolean
  exposeHeaders?: string[]
}

export const cors = (options: CorsOptions = {}) => {
  return async (c: Context, next: Next) => {
    // Handle preflight OPTIONS request
    if (c.req.method === 'OPTIONS') {
      const res = new Response(null, { status: 204 })
      
      // Set CORS headers
      setCorsHeaders(res, options)
      
      return res
    }
    
    // Continue to handler
    await next()
    
    // Set CORS headers on response
    setCorsHeaders(c.res, options)
  }
}

function setCorsHeaders(response: Response, options: CorsOptions) {
  const origin = options.origin ?? '*'
  
  if (Array.isArray(origin)) {
    // Match against allowed origins
    const requestOrigin = response.headers.get('Origin')
    if (requestOrigin && origin.includes(requestOrigin)) {
      response.headers.set('Access-Control-Allow-Origin', requestOrigin)
    }
  } else {
    response.headers.set('Access-Control-Allow-Origin', origin)
  }
  
  // Other CORS headers
  if (options.allowMethods) {
    response.headers.set(
      'Access-Control-Allow-Methods',
      options.allowMethods.join(', ')
    )
  }
  
  if (options.allowHeaders) {
    response.headers.set(
      'Access-Control-Allow-Headers',
      options.allowHeaders.join(', ')
    )
  }
  
  if (options.maxAge) {
    response.headers.set(
      'Access-Control-Max-Age',
      options.maxAge.toString()
    )
  }
  
  if (options.credentials) {
    response.headers.set('Access-Control-Allow-Credentials', 'true')
  }
}
```

### JWT Middleware

```typescript
// src/middleware/jwt/index.ts

interface JwtOptions {
  secret: string
  token?: string  // Where to find token (header, cookie, query)
  alg?: string
}

export const jwt = (options: JwtOptions) => {
  return async (c: Context, next: Next) => {
    // Get token from request
    const token = getTokenFromRequest(c, options)
    
    if (!token) {
      return c.json({ error: 'Unauthorized' }, 401)
    }
    
    try {
      // Verify token
      const payload = await verifyJwt(token, options.secret, options.alg)
      
      // Store in context
      c.set('jwtPayload', payload)
      
      await next()
    } catch (err) {
      return c.json({ error: 'Invalid token' }, 401)
    }
  }
}

function getTokenFromRequest(c: Context, options: JwtOptions): string | null {
  // Default: check Authorization header
  const auth = c.req.header('Authorization')
  if (auth && auth.startsWith('Bearer ')) {
    return auth.slice(7)
  }
  
  // Check cookie
  const cookie = c.req.header('Cookie')
  if (cookie) {
    const match = cookie.match(/token=([^;]+)/)
    if (match) return match[1]
  }
  
  // Check query param
  return c.req.query('token')
}

async function verifyJwt(
  token: string,
  secret: string,
  alg: string = 'HS256'
): Promise<any> {
  // JWT verification implementation
  const encoder = new TextEncoder()
  const key = await crypto.subtle.importKey(
    'raw',
    encoder.encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['verify']
  )
  
  const parts = token.split('.')
  if (parts.length !== 3) throw new Error('Invalid JWT')
  
  const [header, payload, signature] = parts
  
  // Verify signature
  const valid = await crypto.subtle.verify(
    'HMAC',
    key,
    base64UrlDecode(signature),
    encoder.encode(`${header}.${payload}`)
  )
  
  if (!valid) throw new Error('Invalid signature')
  
  return JSON.parse(base64UrlDecode(payload))
}
```

### Body Limit Middleware

```typescript
// src/middleware/body-limit/index.ts

interface BodyLimitOptions {
  maxSize: number  // in bytes
  onError?: (c: Context) => Response
}

export const bodyLimit = (options: BodyLimitOptions) => {
  return async (c: Context, next: Next) => {
    const contentLength = c.req.header('Content-Length')
    
    if (contentLength) {
      const size = parseInt(contentLength, 10)
      
      if (size > options.maxSize) {
        if (options.onError) {
          return options.onError(c)
        }
        return c.json(
          { error: `Payload too large. Max size is ${options.maxSize} bytes` },
          413
        )
      }
    }
    
    await next()
  }
}

// Usage:
app.post('/upload', bodyLimit({ maxSize: 10 * 1024 * 1024 }), async (c) => {
  const formData = await c.req.formData()
  // Handle upload
})
```

## Custom Middleware Patterns

### Request ID Middleware

```typescript
// Custom middleware with context storage

const requestId = async (c: Context, next: Next) => {
  // Get or generate request ID
  let id = c.req.header('X-Request-ID')
  
  if (!id) {
    id = crypto.randomUUID()
  }
  
  // Store in context for downstream access
  c.set('requestId', id)
  
  // Add to response headers
  c.header('X-Request-ID', id)
  
  await next()
}

// Usage in handler
app.get('/status', requestId, async (c) => {
  const requestId = c.get('requestId')
  console.log(`Processing request ${requestId}`)
  
  return c.json({ status: 'ok', requestId })
})
```

### Timing Middleware

```typescript
// Factory middleware with options

interface TimingOptions {
  total?: boolean
  handlers?: boolean
}

const timing = (options: TimingOptions = {}) => {
  return async (c: Context, next: Next) => {
    const start = Date.now()
    const timings: Record<string, number> = {}
    
    if (options.total !== false) {
      // Track total time
      await next()
      timings.total = Date.now() - start
    } else {
      await next()
    }
    
    // Add Server-Timing header
    const timingHeader = Object.entries(timings)
      .map(([name, duration]) => `${name};dur=${duration}`)
      .join(', ')
    
    c.header('Server-Timing', timingHeader)
  }
}
```

### Cache Middleware

```typescript
// Cache middleware with storage

interface CacheOptions {
  ttl: number  // Time to live in seconds
  storage?: Map<string, { body: any; expiry: number }>
}

const cache = (options: CacheOptions) => {
  const storage = options.storage ?? new Map()
  
  return async (c: Context, next: Next) => {
    // Only cache GET requests
    if (c.req.method !== 'GET') {
      await next()
      return
    }
    
    const cacheKey = `${c.req.method}:${c.req.url}`
    
    // Check cache
    const cached = storage.get(cacheKey)
    if (cached && cached.expiry > Date.now()) {
      // Return cached response
      c.res = cached.body
      c.res.headers.set('X-Cache', 'HIT')
      return
    }
    
    // Generate response
    await next()
    
    // Store in cache
    storage.set(cacheKey, {
      body: c.res.clone(),  // Clone for storage
      expiry: Date.now() + options.ttl * 1000,
    })
    
    c.res.headers.set('X-Cache', 'MISS')
  }
}
```

## Error Handling

```typescript
// Error handling in composition

const compose = <T extends Context>(
  middleware: Middleware<T>[],
  onError?: (err: Error, c: T) => Response | Promise<Response>,
  onNotFound?: (c: T) => Response
) => {
  return async (c: T, handler: (c: T) => Response) => {
    const next = async (index: number) => {
      if (index >= middleware.length) {
        return handler(c)
      }
      
      try {
        await middleware[index](c, () => next(index + 1))
      } catch (err) {
        if (onError) {
          return onError(err as Error, c)
        }
        throw err
      }
    }
    
    return next(0)
  }
}

// Error handler example
const errorHandler = async (err: Error, c: Context) => {
  console.error('Unhandled error:', err)
  
  // Return appropriate response based on error type
  if (err instanceof HttpError) {
    return c.json({ error: err.message }, err.status)
  }
  
  // Generic error for unknown errors
  return c.json({ error: 'Internal Server Error' }, 500)
}

// Usage with Hono
const app = new Hono()

app.onError((err, c) => {
  return errorHandler(err, c)
})
```

## Conclusion

Hono's middleware composition system provides:

1. **Koa-Style Composition**: Async middleware with before/after execution
2. **Context Propagation**: Shared state between middleware via Context
3. **Type Safety**: Typed context variables for IDE support
4. **Error Handling**: Centralized error handling in composition
5. **Flexible Patterns**: Factory middleware, conditional middleware
6. **Response Modification**: Middleware can modify response after handler

The compose function creates a recursive Promise chain where each middleware can execute code both before and after calling next(), enabling powerful request/response processing patterns.
