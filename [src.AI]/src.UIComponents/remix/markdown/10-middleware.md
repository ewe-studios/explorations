# Middleware: Async Context, Compression, Method Override, Static Files, Session, Form Data, Logger

Seven middleware packages for `@remix-run/fetch-router`. Each implements the `Middleware` interface: `(context, next) => Response | void | Promise<...>`.

Source: 7 packages under `packages/*/middleware/`.

---

## Async Context Middleware

Source: `packages/async-context-middleware/src/lib/async-context.ts` — 28 lines.

```typescript
import { AsyncLocalStorage } from 'node:async_hooks'

const storage = new AsyncLocalStorage<RequestContext>()

export function asyncContext(): Middleware {
  return (context, next) => storage.run(context, next)
}

export function getContext(): RequestContext {
  let context = storage.getStore()
  if (context == null) {
    throw new Error('No request context found. Make sure the asyncContext middleware is installed.')
  }
  return context
}
```

### How It Works

Node.js `AsyncLocalStorage` creates a per-async-execution-context storage. When `storage.run(context, next)` is called, the `context` is available to any code running inside `next()` — even deeply nested functions that don't receive `context` as a parameter.

```typescript
// Route handler — no context parameter needed
async function logSomething() {
  const ctx = getContext()  // Gets the request context from AsyncLocalStorage
  console.log(ctx.url.pathname)
}

// Middleware chain
let router = createRouter({
  middleware: [asyncContext()],  // MUST be first
})

router.map(routes.home, async () => {
  await logSomething()  // Works — context is in AsyncLocalStorage
  return new Response('Home')
})
```

Aha: This middleware is essentially a single line — `storage.run(context, next)`. But it enables a powerful pattern: any function in the call chain can access the request context without it being passed as a parameter. This is critical for logging, error handling, and database access layers that need request information but aren't part of the middleware chain.

### When It Doesn't Work

`getContext()` throws if:
1. The `asyncContext()` middleware is not installed
2. Called outside the async execution context (e.g., in a `setTimeout` callback without `AsyncLocalStorage.snapshot()`)

---

## Compression Middleware

Source: `packages/compression-middleware/src/lib/compression.ts` — 67 lines.

```typescript
type Encoding = 'br' | 'gzip' | 'deflate'

export interface CompressionOptions {
  threshold?: number                           // Default: 1024 bytes
  filterMediaType?: (mediaType: string) => boolean
  encodings?: Encoding[] | ((response: Response) => Encoding[])
  zlib?: ZlibOptions | ((response: Response) => ZlibOptions)
  brotli?: BrotliOptions | ((response: Response) => BrotliOptions)
}

export function compression(options?: CompressionOptions): Middleware {
  return async (context, next) => {
    let response = await next()

    // Skip if no Content-Type
    let contentTypeHeader = response.headers.get('Content-Type')
    if (!contentTypeHeader) return response

    let mediaType = contentTypeHeader.split(';')[0].trim()
    if (!mediaType) return response

    // Skip if not compressible
    let filterMediaType = options?.filterMediaType ?? isCompressibleMimeType
    if (!filterMediaType(mediaType)) return response

    // Build compression options (supports static or per-response functions)
    let compressOptions: CompressResponseOptions = {
      threshold: options?.threshold,
      encodings: typeof options?.encodings === 'function'
        ? options.encodings(response)
        : options?.encodings,
      // ... same pattern for zlib and brotli
    }

    return compressResponse(response, context.request, compressOptions)
  }
}
```

### Compression Flow

```mermaid
flowchart TD
    A[response = await next()] --> B{has Content-Type?}
    B -->|No| C[return response unchanged]
    B -->|Yes| D[extract media type]
    D --> E{compressible?}
    E -->|No| C
    E -->|Yes| F[compressResponse()]
    F --> G{Accept-Encoding matches?}
    G -->|br| H[Brotli compress]
    G -->|gzip| I[gzip compress]
    G -->|deflate| J[deflate compress]
    G -->|none| C
```

### Default Behavior

- **Encodings**: `['br', 'gzip', 'deflate']` — tries Brotli first, then gzip, then deflate
- **Threshold**: 1024 bytes — responses smaller than 1KB are not compressed
- **Filter**: `isCompressibleMimeType` from `@remix-run/mime` — compresses text-based types (HTML, CSS, JS, JSON, XML), skips binary types (images, videos, archives)

### Dynamic Options

All options can be static values or functions that receive the response:

```typescript
compression({
  encodings: (response) => {
    // Use different encodings based on response type
    if (response.headers.get('Content-Type')?.includes('json')) {
      return ['br', 'gzip']  // JSON compresses well with Brotli
    }
    return ['gzip']
  },
  brotli: { params: { [constants.BROTLI_PARAM_QUALITY]: 6 } },
})
```

---

## Method Override Middleware

Source: `packages/method-override-middleware/src/lib/method-override.ts` — 36 lines.

```typescript
export function methodOverride(options?: MethodOverrideOptions): Middleware {
  let fieldName = options?.fieldName ?? '_method'

  return async (context: RequestContext) => {
    let method = context.formData?.get(fieldName)
    if (typeof method !== 'string') return

    let requestMethod = method.toUpperCase() as RequestMethod
    if (RequestMethods.includes(requestMethod)) {
      context.method = requestMethod
    }
  }
}
```

### Why It Exists

HTML `<form>` elements only support `GET` and `POST` methods. To simulate `PUT`, `PATCH`, and `DELETE`, forms include a hidden field:

```html
<form method="POST" action="/users/42">
  <input type="hidden" name="_method" value="PUT">
  <!-- form fields -->
</form>
```

The middleware reads `context.formData.get('_method')` and overwrites `context.method`.

### Placement Requirement

```typescript
let router = createRouter({
  middleware: [
    formData(),        // MUST come first — parses form data
    methodOverride(),  // MUST come after formData
  ],
})
```

The `formData` middleware must be before `methodOverride` because `methodOverride` reads from `context.formData`.

### Validation

Only valid HTTP methods are accepted: `GET`, `HEAD`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`. Invalid values are silently ignored.

---

## Static Files Middleware

Source: `packages/static-middleware/src/lib/static.ts` — 100+ lines.

```typescript
export function staticFiles(root: string, options: StaticFilesOptions = {}): Middleware {
  root = path.resolve(root)  // Ensure absolute path
  let { acceptRanges, filter, index: indexOption, listFiles, ...fileOptions } = options

  let index: string[]
  if (indexOption === false) index = []
  else if (indexOption === true || indexOption === undefined) index = ['index.html', 'index.htm']
  else index = indexOption

  return async (context, next) => {
    if (context.method !== 'GET' && context.method !== 'HEAD') return next()

    let relativePath = context.url.pathname.replace(/^\/+/, '')
    if (filter && !filter(relativePath)) return next()

    let targetPath = path.join(root, relativePath)

    try {
      let stats = await fsp.stat(targetPath)

      if (stats.isFile()) {
        return sendFile(openFile(targetPath), context.request, fileOptions)
      }

      if (stats.isDirectory()) {
        // Try index files
        for (let indexFile of index) {
          let indexPath = path.join(targetPath, indexFile)
          try {
            let indexStats = await fsp.stat(indexPath)
            if (indexStats.isFile()) {
              return sendFile(openFile(indexPath), context.request, fileOptions)
            }
          } catch { /* try next */ }
        }

        // Directory listing
        if (listFiles) {
          return generateDirectoryListing(targetPath, context.url.pathname)
        }
      }
    } catch {
      // File not found — fall through
    }

    return next()
  }
}
```

### Resolution Order

1. **Exact file match**: `/style.css` → `root/style.css`
2. **Index files**: `/docs/` → `root/docs/index.html` → `root/docs/index.htm`
3. **Directory listing**: If `listFiles: true` and no index file found
4. **Fall through**: If nothing matches, calls `next()`

### Accept Ranges

```typescript
acceptRanges?: boolean | AcceptRangesFunction
```

Default: enables ranges only for non-compressible MIME types (videos, archives). Range requests and compression are mutually exclusive — a response with `Accept-Ranges: bytes` will not be compressed.

### Filter

```typescript
filter?: (path: string) => boolean

// Hide dotfiles
staticFiles('./public', {
  filter: (path) => !path.startsWith('.'),
})
```

### Directory Listing

When `listFiles: true`, generates an HTML page listing directory contents with file names, sizes, and modification dates.

---

## Session Middleware

Source: `packages/session-middleware/src/lib/session.ts` — 32 lines.

```typescript
export function session(cookie: Cookie, storage: SessionStorage): Middleware {
  if (!cookie.signed) {
    throw new Error('Session cookie must be signed')
  }

  return async (context, next) => {
    if (context.sessionStarted) {
      throw new Error('Existing session found, refusing to overwrite')
    }

    // Load session from cookie
    let cookieValue = await cookie.parse(context.headers.get('Cookie'))
    let session = await storage.read(cookieValue)
    context.session = session

    // Run downstream
    let response = await next()

    // Save session
    if (session !== context.session) {
      throw new Error('Cannot save session that was initialized by another middleware/handler')
    }

    let setCookieValue = await storage.save(session)
    if (setCookieValue != null) {
      response.headers.set('Set-Cookie', await cookie.serialize(setCookieValue))
    }

    return response
  }
}
```

### Flow

1. **Parse cookie** from `Cookie` request header
2. **Read session** from storage backend (cookie, filesystem, memory)
3. **Attach to context** as `context.session`
4. **Run downstream** middleware and handler
5. **Save session** — writes session data to storage and sets `Set-Cookie` response header

### Guards

- **Signed cookie required**: `cookie.signed` must be true (prevents tampering)
- **No double session**: throws if `context.sessionStarted` is already true
- **Session identity check**: throws if handler replaced `context.session` with a different object

---

## Form Data Middleware

Source: `packages/form-data-middleware/src/lib/form-data.ts` — 37 lines.

```typescript
export function formData(options?: FormDataOptions): Middleware {
  let suppressErrors = options?.suppressErrors ?? false
  let uploadHandler = options?.uploadHandler

  return async (context) => {
    if (context.method === 'GET' || context.method === 'HEAD') return

    let contentType = context.headers.get('Content-Type')
    if (contentType == null ||
        (!contentType.startsWith('multipart/') &&
         !contentType.startsWith('application/x-www-form-urlencoded'))) {
      context.formData = new FormData()
      return
    }

    try {
      context.formData = await parseFormData(context.request, options, uploadHandler)
    } catch (error) {
      if (!suppressErrors || !(error instanceof FormDataParseError)) {
        throw error
      }
      context.formData = new FormData()
    }
  }
}
```

### Content-Type Check

Only parses `multipart/*` and `application/x-www-form-urlencoded` content types. Other types get an empty `FormData`.

### Upload Handler

```typescript
uploadHandler?: FileUploadHandler

// Example: save uploads to disk
formData({
  uploadHandler: async (upload) => {
    const filePath = path.join('/tmp/uploads', upload.filename)
    await fs.writeFile(filePath, upload.stream())
    return { path: filePath }  // Stored in FormData instead of File
  },
})
```

### Error Suppression

With `suppressErrors: true`, parse failures produce an empty `FormData` instead of throwing.

---

## Logger Middleware

Source: `packages/logger-middleware/src/lib/logger.ts` — 90+ lines.

```typescript
export function logger(options: LoggerOptions = {}): Middleware {
  let { format = '[%date] %method %path %status %contentLength', log = console.log } = options

  return async ({ request, url }, next) => {
    let start = new Date()
    let response = await next()
    let end = new Date()

    let tokens: Record<string, () => string> = {
      date: () => formatApacheDate(start),
      dateISO: () => start.toISOString(),
      duration: () => String(end.getTime() - start.getTime()),
      contentLength: () => response.headers.get('Content-Length') ?? '-',
      contentType: () => response.headers.get('Content-Type') ?? '-',
      method: () => request.method,
      path: () => url.pathname + url.search,
      status: () => String(response.status),
      // ... 14 more tokens
    }

    let message = format.replace(/%(\w+)/g, (_, key) => tokens[key]?.() ?? '-')
    log(message)
    return response
  }
}
```

### Apache Date Format

```typescript
function formatApacheDate(date: Date): string {
  // "23/Sep/2025:11:34:12 -0700"
  let timezoneOffset = date.getTimezoneOffset()
  let sign = timezoneOffset <= 0 ? '+' : '-'
  // ...
}
```

### Available Tokens

| Token | Example |
|-------|---------|
| `%date` | `23/Sep/2025:11:34:12 -0700` |
| `%dateISO` | `2025-09-23T11:34:12.000Z` |
| `%duration` | `42` (ms) |
| `%method` | `GET` |
| `%path` | `/users/42?tab=posts` |
| `%status` | `200` |
| `%contentLength` | `1234` |
| `%contentType` | `text/html; charset=UTF-8` |
| `%url` | `https://example.com/users/42` |
| `%userAgent` | `Mozilla/5.0...` |
| `%referer` | `https://google.com` |

### Custom Logger

```typescript
logger({
  format: '%duration ms %method %path %status',
  log: (msg) => winston.info(msg),
})
```
