---
source: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/hono
repository: github.com/honojs/hono
explored_at: 2026-04-05
focus: Context object internals, request/response handling, header management, body parsing, context storage
---

# Deep Dive: Context Object Internals

## Overview

This deep dive examines Hono's Context object - the central piece that wraps the Request, provides response helpers, manages headers, handles body parsing, and enables middleware communication through typed storage.

## Architecture

```mermaid
flowchart TB
    subgraph Request["Request Layer"]
        RawReq[Raw Request]
        HonoReq[HonoRequest Wrapper]
        Params[Route Params]
        Query[Query Params]
        Headers[Request Headers]
    end
    
    subgraph Context["Context Object"]
        Status[Status Code]
        ResHeaders[Response Headers]
        Body[Response Body]
        Variables[Context Variables]
        ExecCtx[ExecutionContext]
    end
    
    subgraph Response["Response Layer"]
        Text[text()]
        JSON[json()]
        HTML[html()]
        Redirect[redirect()]
        Render[render()]
    end
    
    RawReq --> HonoReq
    HonoReq --> Context
    Params --> Context
    Query --> Context
    Headers --> Context
    
    Context --> Status
    Context --> ResHeaders
    Context --> Body
    Context --> Variables
    
    Context --> Text
    Context --> JSON
    Context --> HTML
    Context --> Redirect
    Context --> Render
```

## Context Class Structure

```typescript
// src/context.ts - Full Context implementation

import type { HonoRequest } from './request'
import type { Env, NotFoundHandler, ErrorHandler } from './types'

type HeadersRecord = Record<string, string>

interface ContextOptions {
  env?: any
  executionCtx?: ExecutionContext
  notFoundHandler?: NotFoundHandler
  errorHandler?: ErrorHandler
}

export class Context {
  // === Request Properties ===
  
  /** Raw Request object */
  private rawRequest: Request
  
  /** HonoRequest wrapper with parsing methods */
  req: HonoRequest
  
  /** Current path */
  private path: string
  
  // === Response State ===
  
  /** Response status code */
  private _status: number = 200
  
  /** Response headers (lazy initialization) */
  private _h: Headers | undefined
  
  /** Response body */
  private body: BodyInit | null = null
  
  /** Whether response has been written */
  private _p: boolean = false  // "prepared"
  
  // === Context Storage ===
  
  /** Map for middleware communication */
  private #variables: Map<string, unknown>
  
  // === Execution Context ===
  
  /** ExecutionContext for background tasks */
  private executionCtx: ExecutionContext | undefined
  
  // === Error Handling ===
  
  private notFoundHandler: NotFoundHandler
  private errorHandler: ErrorHandler
  
  constructor(req: Request, options: ContextOptions = {}) {
    this.rawRequest = req
    this.path = new URL(req.url).pathname
    
    // Initialize HonoRequest wrapper
    this.req = new HonoRequest(req, this.path)
    
    // Options
    this.executionCtx = options.executionCtx
    this.notFoundHandler = options.notFoundHandler
    this.errorHandler = options.errorHandler
    
    // Initialize variables map
    this.#variables = new Map()
  }
  
  // === Request Access ===
  
  /** Get request URL */
  get url(): string {
    return this.req.url
  }
  
  /** Get HTTP method */
  get method(): string {
    return this.req.method
  }
  
  /** Get request path */
  get path(): string {
    return this.path
  }
  
  /** Get route parameters */
  param(name: string): string | undefined
  param(): Record<string, string>
  param(name?: string) {
    if (name) {
      return this.req.param(name)
    }
    return this.req.params
  }
  
  /** Get query parameters */
  query(name: string): string | undefined
  query(): Record<string, string>
  query(name?: string) {
    if (name) {
      return this.req.query(name)
    }
    return this.req.query()
  }
  
  /** Get request header */
  header(name: string): string | null {
    return this.req.header(name)
  }
  
  // === Response Headers ===
  
  /** Set response header */
  header(name: string, value: string, options?: { append?: boolean }): void {
    this._h ||= new Headers()
    
    if (options?.append) {
      this._h.append(name, value)
    } else {
      this._h.set(name, value)
    }
  }
  
  /** Get all response headers */
  get resHeaders(): Headers {
    return this._h ?? new Headers()
  }
  
  // === Response Helpers ===
  
  /** Text response */
  text(text: string, arg?: number | HeadersRecord, headers?: HeadersRecord): Response {
    return this.newResponse(text, arg, headers, 'text/plain')
  }
  
  /** JSON response */
  json<T>(object: T, arg?: number | HeadersRecord, headers?: HeadersRecord): Response {
    const body = JSON.stringify(object)
    return this.newResponse(body, arg, headers, 'application/json')
  }
  
  /** HTML response */
  html(html: string, arg?: number | HeadersRecord, headers?: HeadersRecord): Response {
    return this.newResponse(html, arg, headers, 'text/html')
  }
  
  /** Redirect response */
  redirect(location: string, status: number = 302): Response {
    const headers = new Headers()
    headers.set('Location', location)
    return new Response(null, { status, headers })
  }
  
  /** Render JSX component */
  render(template: FC<Props>, props: Props): Response {
    const html = renderToString(template(props))
    return this.html(html)
  }
  
  /** Raw body response */
  body(
    data: BodyInit | null,
    arg?: number | HeadersRecord,
    headers?: HeadersRecord
  ): Response {
    return this.newResponse(data, arg, headers)
  }
  
  // === Context Storage ===
  
  /** Set a value in context (for middleware communication) */
  set<Key extends keyof ContextVariableMap>(key: Key, value: ContextVariableMap[Key]): void
  set(key: string, value: unknown): void
  set(key: string, value: unknown): void {
    this.#variables.set(key, value)
  }
  
  /** Get a value from context */
  get<Key extends keyof ContextVariableMap>(key: Key): ContextVariableMap[Key] | undefined
  get<T>(key: string): T | undefined
  get(key: string): unknown | undefined {
    return this.#variables.get(key)
  }
  
  /** Check if key exists in context */
  has(key: string): boolean {
    return this.#variables.has(key)
  }
  
  /** Delete a value from context */
  delete(key: string): boolean {
    return this.#variables.delete(key)
  }
  
  // === Internal Response Building ===
  
  /** Create new response with proper status and headers */
  private newResponse(
    data: BodyInit | null,
    arg?: number | HeadersRecord,
    headers?: HeadersRecord,
    contentType?: string
  ): Response {
    // Handle status code
    let status = this._status
    if (typeof arg === 'number') {
      status = arg
    }
    
    // Build headers
    const responseHeaders = new Headers(this._h)
    
    // Add provided headers
    if (headers) {
      for (const [key, value] of Object.entries(headers)) {
        responseHeaders.set(key, value)
      }
    }
    
    // Add content type if specified
    if (contentType && !responseHeaders.has('Content-Type')) {
      responseHeaders.set('Content-Type', contentType)
    }
    
    // Create and return response
    return new Response(data, {
      status,
      headers: responseHeaders,
    })
  }
  
  /** Clone context for sub-requests */
  clone(): Context {
    const cloned = new Context(this.rawRequest, {
      executionCtx: this.executionCtx,
      notFoundHandler: this.notFoundHandler,
      errorHandler: this.errorHandler,
    })
    cloned.path = this.path
    cloned._status = this._status
    cloned._h = this._h ? new Headers(this._h) : undefined
    cloned.body = this.body
    cloned.#variables = new Map(this.#variables)
    return cloned
  }
}
```

## HonoRequest Wrapper

```typescript
// src/request.ts - Request wrapper

export class HonoRequest {
  private rawRequest: Request
  private path: string
  
  // Cached values
  private cachedQuery: Record<string, string> | undefined
  private cachedParams: Record<string, string> = {}
  
  constructor(rawRequest: Request, path: string) {
    this.rawRequest = rawRequest
    this.path = path
  }
  
  // === URL Properties ===
  
  get url(): string {
    return this.rawRequest.url
  }
  
  get method(): string {
    return this.rawRequest.method
  }
  
  get headers(): Headers {
    return this.rawRequest.headers
  }
  
  get bodyUsed(): boolean {
    return this.rawRequest.bodyUsed
  }
  
  // === Body Parsing ===
  
  async json(): Promise<any> {
    return this.rawRequest.json()
  }
  
  async text(): Promise<string> {
    return this.rawRequest.text()
  }
  
  async formData(): Promise<FormData> {
    return this.rawRequest.formData()
  }
  
  async arrayBuffer(): Promise<ArrayBuffer> {
    return this.rawRequest.arrayBuffer()
  }
  
  async blob(): Promise<Blob> {
    return this.rawRequest.blob()
  }
  
  // === Query Parameters ===
  
  query(name?: string): string | undefined | Record<string, string> {
    if (!this.cachedQuery) {
      this.cachedQuery = this.parseQuery()
    }
    
    if (name) {
      return this.cachedQuery[name]
    }
    return this.cachedQuery
  }
  
  private parseQuery(): Record<string, string> {
    const url = new URL(this.rawRequest.url)
    const result: Record<string, string> = {}
    
    for (const [key, value] of url.searchParams) {
      result[key] = value
    }
    
    return result
  }
  
  // === Route Parameters ===
  
  param(name?: string): string | undefined | Record<string, string> {
    if (name) {
      return this.cachedParams[name]
    }
    return this.cachedParams
  }
  
  /** Set route parameters (called by router) */
  setParams(params: Record<string, string>): void {
    this.cachedParams = params
  }
  
  // === Header Access ===
  
  header(name: string): string | null {
    return this.rawRequest.headers.get(name)
  }
  
  /** Get all headers as object */
  allHeaders(): Record<string, string> {
    const result: Record<string, string> = {}
    this.rawRequest.headers.forEach((value, key) => {
      result[key] = value
    })
    return result
  }
}
```

## Body Parsing Implementation

```typescript
// src/utils/body.ts - Body parsing utilities

/**
 * Parse request body based on Content-Type
 */
export async function parseBody(c: Context): Promise<unknown> {
  const contentType = c.req.header('Content-Type')
  
  if (!contentType) {
    return c.req.text()
  }
  
  // JSON
  if (contentType.startsWith('application/json')) {
    return c.req.json()
  }
  
  // Form data
  if (contentType.startsWith('multipart/form-data') ||
      contentType.startsWith('application/x-www-form-urlencoded')) {
    return c.req.formData()
  }
  
  // Text
  if (contentType.startsWith('text/')) {
    return c.req.text()
  }
  
  // Default: array buffer
  return c.req.arrayBuffer()
}

/**
 * Parse form data to object
 */
export async function parseFormData(formData: FormData): Promise<Record<string, any>> {
  const result: Record<string, any> = {}
  
  for (const [key, value] of formData.entries()) {
    // Handle multiple values for same key
    if (result.hasOwnProperty(key)) {
      if (!Array.isArray(result[key])) {
        result[key] = [result[key]]
      }
      result[key].push(value)
    } else {
      result[key] = value
    }
  }
  
  return result
}

/**
 * Get body value by key (for form data)
 */
export async function getBodyValue(
  formData: FormData,
  key: string
): Promise<string | File | undefined> {
  const value = formData.get(key)
  return value ?? undefined
}

/**
 * Body limit checker
 */
export function checkBodyLimit(
  contentLength: string | null,
  maxSize: number
): boolean {
  if (!contentLength) return true
  
  const size = parseInt(contentLength, 10)
  return size <= maxSize
}
```

## Header Management

```typescript
// src/utils/headers.ts - Header utilities

/**
 * Merge headers from multiple sources
 */
export function mergeHeaders(
  ...sources: (Headers | HeadersInit | undefined)[]
): Headers {
  const result = new Headers()
  
  for (const source of sources) {
    if (!source) continue
    
    if (source instanceof Headers) {
      source.forEach((value, key) => {
        result.set(key, value)
      })
    } else if (Array.isArray(source)) {
      for (const [key, value] of source) {
        result.set(key, value)
      }
    } else {
      for (const [key, value] of Object.entries(source)) {
        result.set(key, value)
      }
    }
  }
  
  return result
}

/**
 * Common security headers
 */
export const securityHeaders = {
  'X-Content-Type-Options': 'nosniff',
  'X-Frame-Options': 'DENY',
  'X-XSS-Protection': '1; mode=block',
  'Referrer-Policy': 'strict-origin-when-cross-origin',
}

/**
 * Add security headers to response
 */
export function addSecurityHeaders(headers: Headers): void {
  for (const [key, value] of Object.entries(securityHeaders)) {
    if (!headers.has(key)) {
      headers.set(key, value)
    }
  }
}

/**
 * CORS header presets
 */
export const corsPresets = {
  // Allow all (development)
  allowAll: {
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
  },
  
  // Restricted (production)
  restricted: (allowedOrigins: string[]) => ({
    'Access-Control-Allow-Origin': allowedOrigins.join(', '),
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, Authorization',
    'Access-Control-Allow-Credentials': 'true',
  }),
}
```

## Context Variables (Type-Safe)

```typescript
// Type-safe context variables

// Define your context variables
interface ContextVariableMap {
  user: { id: number; email: string; role: string }
  db: Database
  requestId: string
  startTime: number
}

// Extend Context type with your variables
type AppContext = Context<{ Variables: ContextVariableMap }>

// Middleware that sets variables
const authMiddleware = async (c: AppContext, next: Next) => {
  const token = c.req.header('Authorization')?.replace('Bearer ', '')
  
  if (token) {
    try {
      const user = await validateToken(token)
      c.set('user', user)  // Type-safe!
    } catch (err) {
      // Invalid token
    }
  }
  
  await next()
}

const dbMiddleware = async (c: AppContext, next: Next) => {
  const db = new Database()
  c.set('db', db)
  
  try {
    await next()
  } finally {
    await db.close()  // Cleanup
  }
}

// Handler with typed access
const handler = async (c: AppContext) => {
  // Type inference works!
  const user = c.get('user')  // Type: { id: number; email: string; role: string } | undefined
  const db = c.get('db')      // Type: Database | undefined
  
  if (!user) {
    return c.json({ error: 'Unauthorized' }, 401)
  }
  
  const posts = await db.query('SELECT * FROM posts WHERE user_id = ?', [user.id])
  return c.json({ posts })
}
```

## Response Building Flow

```typescript
// Flow from handler call to Response object

class Context {
  // 1. Handler calls response method
  handler(c: Context) {
    // User calls json()
    return c.json({ message: 'Hello' }, 200, { 'X-Custom': 'value' })
  }
  
  // 2. json() method implementation
  json<T>(object: T, arg?: number | HeadersRecord, headers?: HeadersRecord): Response {
    const body = JSON.stringify(object)
    return this.newResponse(body, arg, headers, 'application/json')
  }
  
  // 3. newResponse builds the Response
  private newResponse(
    data: BodyInit | null,
    arg?: number | HeadersRecord,
    headers?: HeadersRecord,
    contentType?: string
  ): Response {
    // Determine status
    let status = this._status
    if (typeof arg === 'number') {
      status = arg
    }
    
    // Build headers
    const responseHeaders = new Headers(this._h)
    
    if (headers) {
      for (const [key, value] of Object.entries(headers)) {
        responseHeaders.set(key, value)
      }
    }
    
    if (contentType && !responseHeaders.has('Content-Type')) {
      responseHeaders.set('Content-Type', contentType)
    }
    
    // Return Response
    return new Response(data, { status, headers: responseHeaders })
  }
}

// Final Response:
// status: 200
// headers: { 'Content-Type': 'application/json', 'X-Custom': 'value' }
// body: '{"message":"Hello"}'
```

## ExecutionContext Usage

```typescript
// Using ExecutionContext for background tasks

class Context {
  /**
   * ExecutionContext allows background tasks after response
   */
  get executionCtx(): ExecutionContext | undefined {
    return this.executionCtx
  }
  
  /**
   * Schedule work that runs after response is sent
   */
  waitUntil(promise: Promise<void>): void {
    this.executionCtx?.waitUntil(promise)
  }
  
  /**
   * Extend lifetime of execution
   */
  passThroughOnException(): void {
    this.executionCtx?.passThroughOnException()
  }
}

// Usage example
app.post('/send-email', async (c) => {
  const emailData = await c.req.json()
  
  // Schedule email sending (doesn't block response)
  c.waitUntil(sendEmail(emailData))
  
  // Return immediately
  return c.json({ status: 'queued' })
})

async function sendEmail(data: any): Promise<void> {
  // This runs in background after response is sent
  await smtpClient.send(data)
}
```

## Conclusion

The Context object is Hono's central abstraction:

1. **Request Wrapping**: HonoRequest provides convenient parsing methods
2. **Response Helpers**: text(), json(), html(), redirect(), render()
3. **Header Management**: Lazy Headers initialization with merge utilities
4. **Body Parsing**: Content-Type aware parsing with caching
5. **Context Storage**: Type-safe variable map for middleware communication
6. **ExecutionContext**: Background task scheduling via waitUntil()

The Context enables clean separation between request processing (middleware) and response generation (handlers) while providing type-safe data sharing throughout the chain.
