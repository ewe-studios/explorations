---
source: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/hono
repository: github.com/honojs/hono
explored_at: 2026-04-05
focus: Runtime adapter implementations, Cloudflare Workers, Deno, Bun, Node.js, Lambda, Vercel deployment
---

# Deep Dive: Runtime Adapters

## Overview

This deep dive examines Hono's runtime adapter system - how the same code runs across Cloudflare Workers, Deno, Bun, Node.js, AWS Lambda, Lambda@Edge, and Vercel. We'll explore how adapters normalize different runtime APIs to Web Standards.

## Architecture

```mermaid
flowchart TB
    subgraph Hono["Hono Core"]
        App[Hono App]
        Router[Router]
        Context[Context]
    end
    
    subgraph Adapters["Runtime Adapters"]
        CF[Cloudflare Workers]
        Deno[Deno]
        Bun[Bun]
        Node[Node.js]
        Lambda[AWS Lambda]
        Edge[Lambda@Edge]
        Vercel[Vercel]
    end
    
    subgraph Runtimes["JavaScript Runtimes"]
        Workers[Workers Runtime]
        DenoRuntime[Deno Runtime]
        BunRuntime[Bun Runtime]
        NodeRuntime[Node.js Runtime]
        LambdaRuntime[Lambda Runtime]
    end
    
    App --> CF
    App --> Deno
    App --> Bun
    App --> Node
    App --> Lambda
    App --> Edge
    App --> Vercel
    
    CF --> Workers
    Deno --> DenoRuntime
    Bun --> BunRuntime
    Node --> NodeRuntime
    Lambda --> LambdaRuntime
    Edge --> LambdaRuntime
    Vercel --> LambdaRuntime
```

## Web Standards Foundation

```typescript
// All adapters target Web Standards APIs

// Request/Response are universal
const request = new Request('http://example.com/')
const response = new Response('Hello')

// Hono works with these directly
app.fetch(request) // Returns Promise<Response>

// Adapters only need to convert:
// Runtime-specific -> Web Standards
// Web Standards -> Runtime-specific
```

## Cloudflare Workers Adapter

```typescript
// src/adapter/cloudflare-workers/index.ts

import type { Hono } from '../../hono'
import type { ExecutionContext } from '@cloudflare/workers-types'

interface CloudflareEnv {
  Bindings: {
    // Cloudflare Workers bindings
    DB: D1Database
    KV: KVNamespace
    R2: R2Bucket
    [key: string]: any
  }
}

/**
 * Handle Cloudflare Workers request
 */
export const handle = <E extends CloudflareEnv>(app: Hono<E>) => {
  return {
    // Workers fetch handler
    fetch: async (
      request: Request,
      env: E['Bindings'],
      executionCtx: ExecutionContext
    ): Promise<Response> => {
      // Create context with Workers-specific options
      const contextOptions = {
        env,
        executionCtx,
        notFoundHandler: (c) => c.text('Not Found', 404),
      }
      
      // Hono handles the rest via Web Standards
      return app.fetch(request, contextOptions)
    },
    
    // Scheduled handler (for cron triggers)
    scheduled: async (
      event: ScheduledEvent,
      env: E['Bindings'],
      executionCtx: ExecutionContext
    ) => {
      // Handle scheduled events
      const request = new Request('http://localhost/scheduled', {
        method: 'POST',
        body: JSON.stringify(event),
      })
      
      return app.fetch(request, { env, executionCtx })
    },
  }
}

/**
 * Serve static files from KV
 */
export const serveStatic = (options: {
  namespace: KVNamespace
  root?: string
}) => {
  return async (c: Context, next: Next) => {
    const path = c.req.path.replace(/^\//, '')
    const key = options.root ? `${options.root}/${path}` : path
    
    const value = await options.namespace.get(key, 'arrayBuffer')
    
    if (!value) {
      await next()
      return
    }
    
    const contentType = getMimeType(path)
    return c.body(value, 200, {
      'Content-Type': contentType,
      'Cache-Control': 'public, max-age=31536000',
    })
  }
}

// Usage
import { Hono } from 'hono'
import { handle } from 'hono/cloudflare-workers'

const app = new Hono()

app.get('/', (c) => c.text('Hello from Workers!'))

export default {
  fetch: handle(app).fetch,
}
```

## Deno Adapter

```typescript
// src/adapter/deno/index.ts

import type { Hono } from '../../hono'

interface DenoEnv {
  Variables: {
    // Deno-specific variables
  }
}

/**
 * Serve Hono app on Deno
 */
export const serve = <E extends DenoEnv>(
  app: Hono<E>,
  options: { port?: number; hostname?: string } = {}
) => {
  const port = options.port ?? 8000
  const hostname = options.hostname ?? '0.0.0.0'
  
  const handler = async (request: Request): Promise<Response> => {
    return app.fetch(request, {
      notFoundHandler: (c) => c.text('Not Found', 404),
    })
  }
  
  // Deno.serve uses Web Standards natively
  return Deno.serve({ port, hostname }, handler)
}

/**
 * Static file middleware for Deno
 */
export const serveStatic = (options: { root?: string } = {}) => {
  return async (c: Context, next: Next) => {
    const path = c.req.path
    const filePath = options.root ? `${options.root}${path}` : `.${path}`
    
    try {
      const stat = await Deno.stat(filePath)
      
      if (stat.isFile) {
        const file = await Deno.readFile(filePath)
        const contentType = getMimeType(path)
        return c.body(file, 200, {
          'Content-Type': contentType,
        })
      }
    } catch (e) {
      // File not found, continue
    }
    
    await next()
  }
}

/**
 * WebSocket helper for Deno
 */
export const upgradeWebSocket = (options: {
  onOpen?: (ws: WebSocket) => void
  onMessage?: (ws: WebSocket, data: any) => void
  onClose?: (ws: WebSocket) => void
}) => {
  return async (c: Context, next: Next) => {
    const { response, socket } = Deno.upgradeWebSocket(c.req.raw)
    
    if (options.onOpen) {
      socket.onopen = () => options.onOpen!(socket)
    }
    
    if (options.onMessage) {
      socket.onmessage = (event) => options.onMessage!(socket, event.data)
    }
    
    if (options.onClose) {
      socket.onclose = () => options.onClose!(socket)
    }
    
    return response
  }
}

// Usage
import { Hono } from 'hono'
import { serve } from 'hono/deno'

const app = new Hono()

app.get('/', (c) => c.text('Hello from Deno!'))

serve(app, { port: 8000 })
```

## Bun Adapter

```typescript
// src/adapter/bun/index.ts

import type { Hono } from '../../hono'

interface BunEnv {
  Variables: {
    // Bun-specific variables
  }
}

/**
 * Serve Hono app on Bun
 */
export const serve = <E extends BunEnv>(
  app: Hono<E>,
  options: { port?: number; hostname?: string } = {}
) => {
  const port = options.port ?? 3000
  const hostname = options.hostname ?? '0.0.0.0'
  
  const handler = async (request: Request): Promise<Response> => {
    return app.fetch(request, {
      notFoundHandler: (c) => c.text('Not Found', 404),
    })
  }
  
  // Bun.serve uses Web Standards natively
  return Bun.serve({
    port,
    hostname,
    fetch: handler,
  })
}

/**
 * Static file middleware for Bun
 */
export const serveStatic = (options: { root?: string } = {}) => {
  return async (c: Context, next: Next) => {
    const path = c.req.path
    const filePath = options.root ? `${options.root}${path}` : `.${path}`
    
    try {
      const file = Bun.file(filePath)
      const exists = await file.exists()
      
      if (exists) {
        return c.body(file.stream(), 200, {
          'Content-Type': file.type,
        })
      }
    } catch {
      // File not found, continue
    }
    
    await next()
  }
}

/**
 * File upload handling with Bun
 */
export const handleMultipart = async (c: Context) => {
  const formData = await c.req.formData()
  const files: File[] = []
  
  for (const [key, value] of formData.entries()) {
    if (value instanceof File) {
      files.push(value)
      
      // Save file using Bun
      const path = `./uploads/${value.name}`
      await Bun.write(path, value.stream())
    }
  }
  
  return c.json({ uploaded: files.map(f => f.name) })
}

// Usage
import { Hono } from 'hono'
import { serve } from 'hono/bun'

const app = new Hono()

app.get('/', (c) => c.text('Hello from Bun!'))

serve(app, { port: 3000 })
```

## Node.js Adapter

```typescript
// src/adapter/node-server/index.ts

import type { Hono } from '../../hono'
import * as http from 'http'
import * as https from 'https'

interface NodeEnv {
  Variables: {
    // Node.js-specific variables
  }
}

/**
 * Convert Node IncomingMessage to Web Request
 */
function toWebRequest(req: http.IncomingMessage): Request {
  const protocol = req.socket.encrypted ? 'https' : 'http'
  const host = req.headers.host || 'localhost'
  const url = `${protocol}://${host}${req.url}`
  
  // Convert headers
  const headers: HeadersInit = {}
  for (const [key, value] of Object.entries(req.headers)) {
    if (value) {
      headers[key] = Array.isArray(value) ? value.join(', ') : value
    }
  }
  
  // Get body
  let body: BodyInit | null = null
  if (req.method !== 'GET' && req.method !== 'HEAD') {
    body = new Uint8Array()
  }
  
  return new Request(url, {
    method: req.method,
    headers,
    body: body as any,
  })
}

/**
 * Convert Web Response to Node response
 */
function sendNodeResponse(
  res: http.ServerResponse,
  webResponse: Response
) {
  // Set status
  res.statusCode = webResponse.status
  res.statusMessage = webResponse.statusText
  
  // Set headers
  for (const [key, value] of webResponse.headers.entries()) {
    res.setHeader(key, value)
  }
  
  // Send body
  if (webResponse.body) {
    const reader = webResponse.body.getReader()
    
    const send = async () => {
      try {
        while (true) {
          const { done, value } = await reader.read()
          if (done) break
          res.write(value)
        }
        res.end()
      } catch (err) {
        res.end()
      }
    }
    
    send()
  } else {
    res.end()
  }
}

/**
 * Create HTTP server for Hono
 */
export const createServer = <E extends NodeEnv>(app: Hono<E>) => {
  const handleRequest = async (
    req: http.IncomingMessage,
    res: http.ServerResponse
  ) => {
    try {
      // Convert to Web Request
      const webRequest = toWebRequest(req)
      
      // Let Hono handle it
      const webResponse = await app.fetch(webRequest, {
        notFoundHandler: (c) => c.text('Not Found', 404),
      })
      
      // Convert back to Node response
      sendNodeResponse(res, webResponse)
    } catch (err) {
      console.error(err)
      res.statusCode = 500
      res.end('Internal Server Error')
    }
  }
  
  return http.createServer(handleRequest)
}

/**
 * Serve Hono app on Node.js
 */
export const serve = <E extends NodeEnv>(
  app: Hono<E>,
  options: { port?: number; hostname?: string } = {}
) => {
  const port = options.port ?? 3000
  const hostname = options.hostname ?? '0.0.0.0'
  
  const server = createServer(app)
  
  server.listen(port, hostname, () => {
    console.log(`Server running on http://${hostname}:${port}`)
  })
  
  return server
}

/**
 * Static file middleware for Node.js
 */
export const serveStatic = (options: { root?: string } = {}) => {
  return async (c: Context, next: Next) => {
    const path = c.req.path
    const filePath = options.root ? `${options.root}${path}` : `.${path}`
    
    try {
      const stat = await fs.stat(filePath)
      
      if (stat.isFile()) {
        const stream = fs.createReadStream(filePath)
        const contentType = getMimeType(path)
        
        return c.body(stream as any, 200, {
          'Content-Type': contentType,
        })
      }
    } catch {
      // File not found, continue
    }
    
    await next()
  }
}

// Usage
import { Hono } from 'hono'
import { serve } from '@hono/node-server'

const app = new Hono()

app.get('/', (c) => c.text('Hello from Node.js!'))

serve(app, { port: 3000 })
```

## AWS Lambda Adapter

```typescript
// src/adapter/aws-lambda/index.ts

import type { Hono } from '../../hono'
import type {
  APIGatewayProxyEvent,
  APIGatewayProxyResult,
  Context as LambdaContext,
} from 'aws-lambda'

interface LambdaEnv {
  Variables: {
    // Lambda-specific variables
  }
}

/**
 * Convert API Gateway event to Web Request
 */
function toWebRequest(event: APIGatewayProxyEvent): Request {
  const protocol = event.headers['X-Forwarded-Proto'] || 'https'
  const host = event.headers['Host'] || event.requestContext.domainName
  const path = event.path
  const queryString = event.queryStringParameters ? 
    new URLSearchParams(event.queryStringParameters).toString() : ''
  
  const url = `${protocol}://${host}${path}${queryString ? '?' + queryString : ''}`
  
  // Convert headers
  const headers: HeadersInit = {}
  for (const [key, value] of Object.entries(event.headers)) {
    if (value) {
      headers[key] = value
    }
  }
  
  // Get body
  let body: BodyInit | null = null
  if (event.body) {
    body = event.isBase64Encoded 
      ? Buffer.from(event.body, 'base64')
      : event.body
  }
  
  return new Request(url, {
    method: event.httpMethod,
    headers,
    body: body as any,
  })
}

/**
 * Convert Web Response to API Gateway result
 */
function toProxyResult(webResponse: Response): APIGatewayProxyResult {
  const headers: Record<string, string> = {}
  
  for (const [key, value] of webResponse.headers.entries()) {
    headers[key] = value
  }
  
  return {
    statusCode: webResponse.status,
    headers,
    body: await webResponse.text(),
    isBase64Encoded: false,
  }
}

/**
 * Handle Lambda API Gateway request
 */
export const handle = <E extends LambdaEnv>(app: Hono<E>) => {
  return async (
    event: APIGatewayProxyEvent,
    context: LambdaContext
  ): Promise<APIGatewayProxyResult> => {
    try {
      // Convert to Web Request
      const webRequest = toWebRequest(event)
      
      // Let Hono handle it
      const webResponse = await app.fetch(webRequest, {
        notFoundHandler: (c) => c.json({ message: 'Not Found' }, 404),
      })
      
      // Convert to API Gateway result
      return toProxyResult(webResponse)
    } catch (err) {
      console.error(err)
      return {
        statusCode: 500,
        body: JSON.stringify({ message: 'Internal Server Error' }),
      }
    }
  }
}

// Usage
import { Hono } from 'hono'
import { handle } from 'hono/aws-lambda'

const app = new Hono()

app.get('/', (c) => c.json({ message: 'Hello from Lambda!' }))

export const handler = handle(app)
```

## Vercel Adapter

```typescript
// src/adapter/vercel/index.ts

import type { Hono } from '../../hono'
import type { VercelRequest, VercelResponse } from '@vercel/node'

interface VercelEnv {
  Variables: {
    // Vercel-specific variables
  }
}

/**
 * Handle Vercel request
 */
export const handle = <E extends VercelEnv>(app: Hono<E>) => {
  return async (req: VercelRequest, res: VercelResponse) => {
    try {
      // Convert to Web Request
      const protocol = req.headers['x-forwarded-proto'] || 'https'
      const host = req.headers.host || 'localhost'
      const url = `${protocol}://${host}${req.url}`
      
      const headers = new Headers()
      for (const [key, value] of Object.entries(req.headers)) {
        if (value) {
          headers.set(key, Array.isArray(value) ? value.join(', ') : value)
        }
      }
      
      let body: BodyInit | null = null
      if (req.method !== 'GET' && req.method !== 'HEAD') {
        body = typeof req.body === 'string' ? req.body : JSON.stringify(req.body)
      }
      
      const webRequest = new Request(url, {
        method: req.method,
        headers,
        body: body as any,
      })
      
      // Let Hono handle it
      const webResponse = await app.fetch(webRequest, {
        notFoundHandler: (c) => c.json({ message: 'Not Found' }, 404),
      })
      
      // Convert to Vercel response
      res.statusCode = webResponse.status
      
      for (const [key, value] of webResponse.headers.entries()) {
        res.setHeader(key, value)
      }
      
      const body = await webResponse.text()
      res.send(body)
    } catch (err) {
      console.error(err)
      res.status(500).json({ message: 'Internal Server Error' })
    }
  }
}

// Usage
// api/index.ts
import { Hono } from 'hono'
import { handle } from 'hono/vercel'

const app = new Hono()

app.get('/', (c) => c.json({ message: 'Hello from Vercel!' }))

export default handle(app)
```

## Adapter Comparison

```typescript
// Feature comparison across adapters

| Adapter | Native Web Standards | Static Files | WebSocket | Streaming |
|---------|---------------------|--------------|-----------|-----------|
| Cloudflare | Yes | KV/R2 | Yes | Yes |
| Deno | Yes | Deno.fs | Yes | Yes |
| Bun | Yes | Bun.file | Yes | Yes |
| Node.js | Conversion needed | fs module | ws package | Yes |
| Lambda | Conversion needed | S3 | No | Limited |
| Vercel | Conversion needed | Vercel Blob | No | Yes |

// Code that runs everywhere
const app = new Hono()

app.get('/', (c) => c.json({ runtime: c.env.RUNTIME }))

// This same app works on all runtimes!
```

## Conclusion

Hono's adapter system demonstrates:

1. **Web Standards First**: Request/Response APIs are universal
2. **Minimal Conversion**: Adapters only convert at the edges
3. **Runtime Features**: Each adapter exposes runtime-specific capabilities
4. **Same Code Everywhere**: Business logic is runtime-agnostic
5. **Type Safety**: Each adapter has proper TypeScript types
6. **Performance**: Direct Web Standards usage where available

The adapter pattern enables true write-once-run-anywhere deployment for edge applications.
