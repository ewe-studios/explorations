---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/hono
source: github.com/honojs/hono
explored_at: 2026-04-05
prerequisites: Basic JavaScript/TypeScript knowledge, HTTP fundamentals, Familiarity with web frameworks
---

# Zero to Hono Developer - Complete Fundamentals

## Table of Contents

1. [What is Hono?](#what-is-hono)
2. [Core Concepts](#core-concepts)
3. [Getting Started](#getting-started)
4. [Your First Application](#your-first-application)
5. [Routing Deep Dive](#routing-deep-dive)
6. [Middleware](#middleware)
7. [Request Validation](#request-validation)
8. [Context Object](#context-object)
9. [JSX and Server Components](#jsx-and-server-components)
10. [HTTP Client (RPC)](#http-client-rpc)
11. [Runtime Adapters](#runtime-adapters)
12. [Deployment Guide](#deployment-guide)

## What is Hono?

**Hono** (Japanese for "flame🔥") is a lightweight, ultrafast web framework built on Web Standards. It works across all major JavaScript runtimes with the same codebase.

### The Problem Hono Solves

Traditional web development:
```
1. Write Express app for Node.js
2. Need different code for Cloudflare Workers
3. Different code for Deno/Bun
4. Rewrite for Lambda@Edge
5. Maintain multiple codebases
```

Hono approach:
```
1. Write Hono app once
2. Deploy to Cloudflare Workers
3. Deploy to Deno/Bun/Node.js
4. Deploy to Lambda@Edge
5. Same codebase everywhere
```

### Key Features

| Feature | Description |
|---------|-------------|
| **Ultrafast** | RegExpRouter with O(1) lookups, 3x faster than Express |
| **Lightweight** | `hono/tiny` is ~13KB with zero dependencies |
| **Multi-runtime** | Works on Cloudflare, Deno, Bun, Node.js, Lambda, Vercel |
| **Type-Safe** | First-class TypeScript with inferred route types |
| **Batteries Included** | Built-in middleware, validators, JSX, HTTP client |
| **Web Standards** | Uses Request/Response APIs, no custom abstractions |

### Hono vs Alternatives

| Framework | Runtime | Bundle Size | Speed | Best For |
|-----------|---------|-------------|-------|----------|
| **Hono** | Any (Edge + Node) | ~13KB | ⚡⚡⚡ | Edge deployment, multi-runtime |
| **Express** | Node.js only | ~200KB | ⚡ | Legacy Node.js apps |
| **Fastify** | Node.js only | ~150KB | ⚡⚡ | High-performance Node.js |
| **Elysia** | Bun primarily | ~50KB | ⚡⚡⚡ | Bun-native apps |
| **Next.js** | Node.js/Vercel | ~200KB+ | ⚡ | React SSR/SSG |

## Core Concepts

### 1. Web Standards Based

Hono uses standard Web APIs:
```typescript
// Request/Response are standard
const request = new Request('http://localhost:3000/')
const response = new Response('Hello')

// Hono works with these directly
app.fetch(request) // Returns Promise<Response>
```

### 2. Middleware Composition

Hono uses Koa-style middleware composition:
```typescript
// Middleware receives context and next function
const middleware = async (c: Context, next: Next) => {
  console.log('Before handler')
  await next()  // Call next middleware
  console.log('After handler')
}
```

### 3. Context Object

The Context (`c`) wraps the request and provides response helpers:
```typescript
app.get('/', (c) => {
  c.req.url           // Request URL
  c.req.method        // HTTP method
  c.req.headers       // Headers
  c.req.param('id')   // Route params
  
  return c.text('Hello')  // Response helper
})
```

### 4. Router Abstraction

Hono separates routing from the core:
```typescript
// Choose your router
import { RegExpRouter } from 'hono/reg-exp-router'
import { TrieRouter } from 'hono/trie-router'

const app = new Hono({ router: new RegExpRouter() })
```

### 5. Type Inference

Routes are automatically typed:
```typescript
const app = new Hono()
  .get('/posts/:id', (c) => c.json({ id: 1 }))

// Types inferred:
// - c.req.param('id') is string
// - Response is { id: number }
```

## Getting Started

### Installation

```bash
# npm
npm install hono

# yarn
yarn add hono

# pnpm
pnpm add hono

# bun
bun add hono

# deno
deno add npm:hono
```

### Quick Start

```typescript
import { Hono } from 'hono'

const app = new Hono()

app.get('/', (c) => {
  return c.text('Hello Hono!')
})

export default app
```

### Project Structure

```
my-app/
├── src/
│   ├── index.ts      # App entry
│   ├── routes/       # Route definitions
│   ├── middleware/   # Custom middleware
│   └── components/   # JSX components
├── package.json
└── tsconfig.json
```

## Your First Application

### Step 1: Basic Setup

```typescript
// src/index.ts
import { Hono } from 'hono'
import { logger } from 'hono/logger'
import { cors } from 'hono/cors'

const app = new Hono()

// Global middleware
app.use('*', logger())
app.use('/api/*', cors())

// Home route
app.get('/', (c) => {
  return c.html(`
    <!DOCTYPE html>
    <html>
      <head><title>My App</title></head>
      <body>
        <h1>Welcome!</h1>
        <a href="/api/users">API</a>
      </body>
    </html>
  `)
})

export default app
```

### Step 2: Adding Routes

```typescript
// src/routes/users.ts
import { Hono } from 'hono'

const users = new Hono()

// Get all users
users.get('/', (c) => {
  return c.json([
    { id: 1, name: 'Alice' },
    { id: 2, name: 'Bob' }
  ])
})

// Get user by ID
users.get('/:id', (c) => {
  const id = c.req.param('id')
  return c.json({ id, name: 'User' })
})

// Create user
users.post('/', async (c) => {
  const body = await c.req.json()
  return c.json({ created: body }, 201)
})

export default users
```

### Step 3: Mounting Routes

```typescript
// src/index.ts
import users from './routes/users'

const app = new Hono()

// Mount routes
app.route('/users', users)
app.route('/api/users', users)
```

### Step 4: Adding Validation

```typescript
import { validator } from 'hono/validator'

app.post('/posts',
  validator('json', (v, c) => {
    if (!v.title) {
      return c.json({ error: 'Title required' }, 400)
    }
    if (v.title.length > 100) {
      return c.json({ error: 'Title too long' }, 400)
    }
    return v  // Return validated data
  }),
  async (c) => {
    const { title } = c.req.valid('json')
    // title is typed as string here
    return c.json({ title })
  }
)
```

## Routing Deep Dive

### Route Parameters

```typescript
// Basic params
app.get('/users/:id', (c) => {
  const id = c.req.param('id')  // string
})

// Multiple params
app.get('/users/:userId/posts/:postId', (c) => {
  const userId = c.req.param('userId')
  const postId = c.req.param('postId')
})

// Wildcard
app.get('/docs/*', (c) => {
  const path = c.req.param('*')  // Remaining path
})
```

### Route Groups

```typescript
// Create a group
const api = new Hono()

api.get('/users', (c) => c.json([]))
api.post('/users', (c) => c.json({}, 201))

// Mount with prefix
app.route('/api', api)
// GET /api/users
// POST /api/users
```

### Method-Specific Routes

```typescript
app.get('/items', getItems)
app.post('/items', createItem)
app.put('/items/:id', updateItem)
app.delete('/items/:id', deleteItem)
app.patch('/items/:id', patchItem)

// All methods
app.all('/webhook', handleWebhook)
```

### Nested Routes

```typescript
const app = new Hono()
const users = new Hono()
const posts = new Hono()

users.get('/', listUsers)
users.get('/:id', getUser)

posts.get('/', listPosts)
posts.get('/:id', getPost)

app.route('/users', users)
app.route('/posts', posts)

// Results in:
// GET /users/
// GET /users/:id
// GET /posts/
// GET /posts/:id
```

## Middleware

### Global Middleware

```typescript
// Apply to all routes
app.use('*', logger())
app.use('*', poweredBy())

// Apply to path prefix
app.use('/api/*', cors())
app.use('/admin/*', basicAuth({ username, password }))
```

### Route-Specific Middleware

```typescript
const auth = async (c: Context, next: Next) => {
  const token = c.req.header('Authorization')
  if (!token) return c.json({ error: 'Unauthorized' }, 401)
  await next()
}

app.get('/protected', auth, (c) => {
  return c.json({ secret: 'data' })
})
```

### Built-in Middleware

```typescript
import { basicAuth } from 'hono/basic-auth'
import { bearerAuth } from 'hono/bearer-auth'
import { bodyLimit } from 'hono/body-limit'
import { cache } from 'hono/cache'
import { compress } from 'hono/compress'
import { cors } from 'hono/cors'
import { csrf } from 'hono/csrf'
import { etag } from 'hono/etag'
import { ipRestriction } from 'hono/ip-restriction'
import { jwt } from 'hono/jwt'
import { timeout } from 'hono/timeout'

// Usage examples
app.use('/api/*', cors())
app.use('/admin/*', basicAuth({ username: 'admin', password: 'secret' }))
app.use('*', bearerAuth({ token: process.env.API_TOKEN }))
app.post('/upload', bodyLimit({ maxSize: 10 * 1024 * 1024 }))  // 10MB
```

### Custom Middleware

```typescript
// Simple middleware
const requestId = async (c: Context, next: Next) => {
  const id = crypto.randomUUID()
  c.set('requestId', id)
  c.header('X-Request-ID', id)
  await next()
}

// Factory middleware (with options)
const timing = (options: { enabled?: boolean } = {}) => {
  return async (c: Context, next: Next) => {
    if (!options.enabled) {
      await next()
      return
    }
    
    const start = Date.now()
    await next()
    const duration = Date.now() - start
    
    c.header('X-Response-Time', `${duration}ms`)
  }
}

// Usage
app.use('*', requestId())
app.use('/api/*', timing({ enabled: true }))
```

## Request Validation

### Validator Middleware

```typescript
import { validator } from 'hono/validator'

// JSON validation
app.post('/posts',
  validator('json', (v, c) => {
    if (!v.title) return c.json({ error: 'Title required' }, 400)
    if (!v.body) return c.json({ error: 'Body required' }, 400)
    return v as { title: string; body: string }
  }),
  (c) => {
    const { title, body } = c.req.valid('json')
    return c.json({ title, body })
  }
)

// Query parameter validation
app.get('/search',
  validator('query', (v, c) => {
    if (!v.q) return c.json({ error: 'Query required' }, 400)
    return v as { q: string }
  }),
  (c) => {
    const { q } = c.req.valid('query')
    return c.json({ results: [] })
  }
)

// Form data validation
app.post('/upload',
  validator('form', (v, c) => {
    const file = v.file as File
    if (!file) return c.json({ error: 'File required' }, 400)
    return v as { file: File }
  }),
  async (c) => {
    const { file } = c.req.valid('form')
    return c.json({ uploaded: file.name })
  }
)
```

### Zod Integration

```typescript
import { z } from 'zod'
import { zValidator } from '@hono/zod-validator'

const postSchema = z.object({
  title: z.string().min(1).max(100),
  body: z.string().min(1),
  tags: z.array(z.string()).optional(),
})

app.post('/posts',
  zValidator('json', postSchema),
  (c) => {
    const validated = c.req.valid('json')
    // Type is inferred: { title: string; body: string; tags?: string[] }
    return c.json(validated)
  }
)
```

## Context Object

### Request Methods

```typescript
// URL and method
c.req.url           // Full URL string
c.req.method        // HTTP method (GET, POST, etc.)
c.req.path          // Path without query string

// Headers
c.req.header()              // All headers
c.req.header('X-Custom')    // Specific header

// Query parameters
c.req.query()               // All query params
c.req.query('page')         // Specific param

// Route parameters
c.req.param()               // All params
c.req.param('id')           // Specific param

// Body parsing
await c.req.json()          // Parse JSON body
await c.req.text()          // Get text body
await c.req.formData()      // Parse form data
await c.req.arrayBuffer()   // Get as ArrayBuffer
await c.req.blob()          // Get as Blob
```

### Response Helpers

```typescript
// Text response
c.text('Hello', 200, { 'X-Custom': 'value' })

// JSON response
c.json({ key: 'value' }, 200, { 'X-Custom': 'value' })

// HTML response
c.html('<h1>Hello</h1>', 200, { 'Content-Type': 'text/html' })

// Redirect
c.redirect('/new-location', 302)

// Custom body
c.body(new Uint8Array([1, 2, 3]), 200, { 'Content-Type': 'application/octet-stream' })

// Render JSX
c.render(<Component />, { title: 'Page' })
```

### Context Values

```typescript
// Set values (available to downstream middleware)
c.set('user', { id: 1, name: 'Alice' })
c.set('db', database)

// Get values
const user = c.get('user')
const db = c.get('db')

// Storage is typed via Variables type
interface AppVariables {
  user: { id: number; name: string }
  db: Database
}

const app = new Hono<{ Variables: AppVariables }>()
```

### ExecutionContext

```typescript
// Access execution context (for background tasks)
app.post('/send-email', async (c) => {
  c.executionCtx.waitUntil(
    sendEmailAsync()  // Won't delay response
  )
  return c.json({ status: 'queued' })
})
```

## JSX and Server Components

### Basic JSX

```typescript
import { Hono } from 'hono'
import { jsx } from 'hono/jsx'

const app = new Hono()

app.get('/', (c) => {
  return c.render(
    <html>
      <head>
        <title>My App</title>
      </head>
      <body>
        <h1>Hello!</h1>
      </body>
    </html>
  )
})
```

### Components with Props

```typescript
type LayoutProps = {
  title: string
  children?: any
}

const Layout = (props: LayoutProps) => (
  <html>
    <head>
      <title>{props.title}</title>
    </head>
    <body>{props.children}</body>
  </html>
)

app.get('/', (c) => {
  return c.render(
    <Layout title="Home">
      <h1>Welcome!</h1>
    </Layout>
  )
})
```

### Server-Side Hooks

```typescript
import { useState, useEffect } from 'hono/jsx/hooks'

const Counter = () => {
  const [count, setCount] = useState(0)
  
  return (
    <div>
      <p>Count: {count}</p>
      <button onclick={() => setCount(count + 1)}>
        Increment
      </button>
    </div>
  )
}
```

### Streaming

```typescript
import { stream } from 'hono/streaming'

app.get('/stream', (c) => {
  return stream(c, async (stream) => {
    await stream.write('Hello\n')
    await stream.sleep(1000)
    await stream.write('World\n')
  })
})

// SSE (Server-Sent Events)
import { streamSSE } from 'hono/streaming'

app.get('/events', (c) => {
  return streamSSE(c, async (stream) => {
    let i = 0
    while (true) {
      await stream.writeSSE({
        data: `Message ${i}`,
        event: 'update',
      })
      await stream.sleep(1000)
      i++
    }
  })
})
```

## HTTP Client (RPC)

### Type-Safe Client

```typescript
// Define app types
type AppType = {
  '/posts': {
    $get: {
      return: { posts: Post[] }
    }
    $post: {
      json: { title: string }
      return: { post: Post }
    }
  }
  '/posts/:id': {
    $get: {
      param: { id: string }
      return: { post: Post }
    }
  }
}

// Create client
import { hc } from 'hono/client'
const client = hc<AppType>('http://localhost:3000')

// Type-safe calls
const posts = await client.posts.$get()
const post = await client.posts[':id'].$get({ param: { id: '1' } })
const created = await client.posts.$post({ json: { title: 'New' } })
```

### Auto-Type Inference

```typescript
// Infer types from actual app
import { hc } from 'hono/client'
const app = new Hono()
  .get('/posts', (c) => c.json({ posts: [] }))
  .post('/posts', (c) => c.json({ post: {} }))

type AppType = typeof app
const client = hc<AppType>('/api')
```

## Runtime Adapters

### Cloudflare Workers

```typescript
// src/index.ts
import { Hono } from 'hono'
import { handle } from 'hono/cloudflare-workers'

const app = new Hono()

app.get('/', (c) => c.text('Hello from Cloudflare!'))

// Wrangler config
export default {
  fetch: handle(app)
}
```

```toml
# wrangler.toml
name = "my-hono-app"
main = "src/index.ts"
compatibility_date = "2024-01-01"
```

### Deno

```typescript
// src/index.ts
import { Hono } from 'hono'
import { serve } from 'hono/deno'

const app = new Hono()

app.get('/', (c) => c.text('Hello from Deno!'))

// Start server
serve(app, { port: 8000 })
```

```bash
deno run --allow-net src/index.ts
```

### Bun

```typescript
// src/index.ts
import { Hono } from 'hono'
import { serve } from 'hono/bun'

const app = new Hono()

app.get('/', (c) => c.text('Hello from Bun!'))

// Start server
serve(app, { port: 3000 })
```

```bash
bun run src/index.ts
```

### Node.js

```typescript
// src/index.ts
import { Hono } from 'hono'
import { serve } from '@hono/node-server'

const app = new Hono()

app.get('/', (c) => c.text('Hello from Node.js!'))

// Start server
serve(app, { port: 3000 })
```

```bash
npm install @hono/node-server
node --loader ts-node/esm src/index.ts
```

### AWS Lambda

```typescript
// src/index.ts
import { Hono } from 'hono'
import { handle } from 'hono/aws-lambda'

const app = new Hono()

app.get('/', (c) => c.json({ hello: 'lambda' }))

// Lambda handler
export const handler = handle(app)
```

### Vercel

```typescript
// api/index.ts
import { Hono } from 'hono'
import { handle } from 'hono/vercel'

const app = new Hono()

app.get('/', (c) => c.text('Hello from Vercel!'))

export default handle(app)
export const GET = handle(app)
```

## Deployment Guide

### Cloudflare Workers

```bash
# Install Wrangler CLI
npm install -g wrangler

# Login
wrangler login

# Deploy
wrangler deploy
```

### Deno Deploy

```bash
# Install Deno Deploy CLI
deno install -A -r -f https://deno.land/x/deploy/deployctl.ts

# Deploy
deployctl deploy --project=my-app src/index.ts
```

### Bun

```bash
# Build (if needed)
bun build src/index.ts --target bun --outdir dist

# Run with Bun
bun run src/index.ts

# Docker deployment
docker run -p 3000:3000 my-bun-app
```

### Docker (Node.js)

```dockerfile
FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .

EXPOSE 3000

CMD ["node", "dist/index.js"]
```

```bash
docker build -t hono-app .
docker run -p 3000:3000 hono-app
```

### AWS Lambda

```yaml
# serverless.yml
service: hono-api

provider:
  name: aws
  runtime: nodejs20.x
  region: us-east-1

functions:
  api:
    handler: dist/index.handler
    events:
      - http:
          path: /
          method: ANY
      - http:
          path: /{proxy+}
          method: ANY
```

```bash
serverless deploy
```

## Conclusion

Hono provides:

1. **Ultrafast Performance**: RegExpRouter with O(1) lookups
2. **Multi-runtime**: Same code on Cloudflare, Deno, Bun, Node.js, Lambda
3. **Type Safety**: Full TypeScript support with inferred types
4. **Lightweight**: ~13KB bundle with zero dependencies
5. **Batteries Included**: Middleware, validators, JSX, HTTP client
6. **Web Standards**: Request/Response based, no custom abstractions

## Next Steps

- [exploration.md](./exploration.md) - Full architecture deep dive
- [01-router-architecture-deep-dive.md](./01-router-architecture-deep-dive.md) - Router internals
- [02-middleware-composition-deep-dive.md](./02-middleware-composition-deep-dive.md) - Middleware system
- [rust-revision.md](./rust-revision.md) - Hono patterns in Rust
- [production-grade.md](./production-grade.md) - Production deployment patterns
