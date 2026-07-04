# Response Helpers: HTML, Redirect, File, and Compression

`@remix-run/response` (v0.2.0) provides typed Response constructors for the Fetch API: HTML with DOCTYPE injection, redirect, file serving, and compression.

Source: `packages/response/src/` — 6 source files.

## createHtmlResponse — DOCTYPE Injection

Source: `lib/html.ts`

```typescript
const DOCTYPE = '<!DOCTYPE html>'

type HtmlBody = string | SafeHtml | Blob | BufferSource | ReadableStream<Uint8Array>

export function createHtmlResponse(body: HtmlBody, init?: ResponseInit): Response {
  let payload: BodyInit = ensureDoctype(body)

  let headers = new Headers(init?.headers)
  if (!headers.has('Content-Type')) {
    headers.set('Content-Type', 'text/html; charset=UTF-8')
  }

  return new Response(payload, { ...init, headers })
}
```

### ensureDoctype() — Body-Type Dispatch

```typescript
function ensureDoctype(body: HtmlBody): BodyInit {
  if (isSafeHtml(body)) {
    let str = String(body)
    return startsWithDoctype(str) ? str : DOCTYPE + str
  }

  if (typeof body === 'string') {
    return startsWithDoctype(body) ? body : DOCTYPE + body
  }

  if (body instanceof Blob) {
    return prependDoctypeToStream(body.stream())
  }

  if (body instanceof ArrayBuffer || ArrayBuffer.isView(body)) {
    let text = new TextDecoder().decode(body)
    return startsWithDoctype(text) ? text : DOCTYPE + text
  }

  if (body instanceof ReadableStream) {
    return prependDoctypeToStream(body)
  }

  return body
}
```

Aha: The function handles **streaming HTML responses**. When the body is a `Blob` or `ReadableStream`, it doesn't buffer the entire body — instead, it creates a new stream that prepends `<!DOCTYPE html>` to the first chunk.

### startsWithDoctype()

```typescript
function startsWithDoctype(str: string): boolean {
  return /^\s*<!doctype html/i.test(str)
}
```

Accepts leading whitespace: `<!DOCTYPE html>`, `  <!DOCTYPE html>`, `\n<!DOCTYPE html>` all match.

### prependDoctypeToStream() — Streaming DOCTYPE Injection

```typescript
function prependDoctypeToStream(stream: ReadableStream<Uint8Array>): ReadableStream<Uint8Array> {
  let doctypeBytes = new TextEncoder().encode(DOCTYPE)
  let reader = stream.getReader()

  return new ReadableStream({
    async start(controller) {
      try {
        let { value: firstChunk, done } = await reader.read()

        if (done) {
          // Empty stream — just send DOCTYPE
          controller.enqueue(doctypeBytes)
          controller.close()
          return
        }

        // Check first chunk for DOCTYPE
        let text = new TextDecoder().decode(firstChunk)
        if (startsWithDoctype(text)) {
          // Already has DOCTYPE — pass through
          controller.enqueue(firstChunk)
        } else {
          // Prepend DOCTYPE
          controller.enqueue(doctypeBytes)
          controller.enqueue(firstChunk)
        }

        // Pipe remaining chunks
        while (true) {
          let { value, done } = await reader.read()
          if (done) { controller.close(); return }
          controller.enqueue(value)
        }
      } catch (e) {
        controller.error(e)
      }
    },

    cancel(reason) {
      reader.cancel(reason)
    },
  })
}
```

Aha: This creates a **transform stream** that only modifies the first chunk. If the first chunk already has a DOCTYPE, it passes through unchanged. If not, the DOCTYPE bytes are enqueued first. Subsequent chunks flow through unmodified. This means the DOCTYPE is added with zero buffering of the rest of the body.

## createRedirectResponse

Source: `lib/redirect.ts`

```typescript
export function createRedirectResponse(
  url: string | URL,
  init?: number | { status?: number; headers?: Record<string, string> },
): Response {
  const status = typeof init === 'number' ? init : init?.status ?? 302
  const headers = new Headers(typeof init === 'object' ? init.headers : undefined)
  headers.set('Location', typeof url === 'string' ? url : url.href)
  return new Response(null, { status, headers })
}
```

### Redirect Status Codes

| Status | Meaning | Method Preservation |
|--------|---------|-------------------|
| 301 | Permanent redirect | Changes POST to GET |
| 302 | Temporary redirect | Changes POST to GET |
| 303 | See other | Changes POST to GET |
| 307 | Temporary redirect | Preserves method |
| 308 | Permanent redirect | Preserves method |

Default is 302.

## createFileResponse

Source: `lib/file.ts`

```typescript
export function createFileResponse(
  file: Blob,
  init?: { filename?: string; disposition?: 'inline' | 'attachment' } & ResponseInit,
): Response {
  const headers = new Headers(init?.headers)

  if (!headers.has('Content-Type') && file.type) {
    headers.set('Content-Type', file.type)
  }

  if (init?.filename) {
    const disposition = init.disposition ?? 'attachment'
    const value = contentDisposition(disposition, init.filename)
    headers.set('Content-Disposition', value)
  }

  if (!headers.has('Content-Length')) {
    headers.set('Content-Length', String(file.size))
  }

  return new Response(file, { ...init, headers })
}
```

### contentDisposition()

```typescript
function contentDisposition(type: 'inline' | 'attachment', filename: string): string {
  // RFC 6266: filename with proper quoting
  if (needsQuoting(filename)) {
    return `${type}; filename="${quote(filename)}"; filename*=UTF-8''${encodeURIComponent(filename)}`
  }
  return `${type}; filename="${filename}"`
}
```

Uses RFC 5987 `filename*=UTF-8''` encoding for non-ASCII filenames. Dual encoding ensures compatibility with old browsers that don't support UTF-8 filenames.

## createCompressedResponse

Source: `lib/compress.ts`

```typescript
export function createCompressedResponse(
  response: Response,
  encodings: CompressionEncoding[],
): Response | null {
  const acceptEncoding = response.headers.get('Accept-Encoding')
  if (!acceptEncoding) return null

  // Find best matching encoding
  for (const encoding of encodings) {
    if (acceptEncoding.includes(encoding)) {
      // Compress the body
      // Set Content-Encoding header
      // Remove Content-Length (compressed size differs)
      // Return new Response
    }
  }

  return null  // No encoding matched
}
```

Aha: When compressing, the `Content-Length` header is removed because the compressed body size differs from the original. Clients that support compression should use `Transfer-Encoding: chunked` or rely on the connection close to determine body length.

## Package Structure

```
response/src/
├── compress.ts        # createCompressedResponse
├── file.ts            # createFileResponse + contentDisposition
├── html.ts            # createHtmlResponse + streaming DOCTYPE injection
├── redirect.ts        # createRedirectResponse
└── lib/
    ├── compress.test.ts
    ├── file.test.ts
    ├── html.test.ts
    └── redirect.test.ts
```

## Cross-References

- [Overview](00-overview.md) — What Remix is
- [Headers](06-headers.md) — HTTP headers (used by response helpers)
