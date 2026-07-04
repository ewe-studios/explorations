# Node Fetch Server: Node.js HTTP Server with Fetch Handler

`@remix-run/node-fetch-server` wraps a `fetch`-compatible handler in a Node.js `http.RequestListener`, converting between Node.js `IncomingMessage`/`ServerResponse` and standard `Request`/`Response`.

Source: `packages/node-fetch-server/src/lib/request-listener.ts` — 150+ lines.

## createRequestListener

```typescript
export function createRequestListener(
  handler: FetchHandler,
  options?: RequestListenerOptions,
): http.RequestListener {
  let onError = options?.onError ?? defaultErrorHandler

  return async (req, res) => {
    let request = createRequest(req, res, options)
    let client = {
      address: req.socket.remoteAddress!,
      family: req.socket.remoteFamily! as ClientAddress['family'],
      port: req.socket.remotePort!,
    }

    let response: Response
    try {
      response = await handler(request, client)
    } catch (error) {
      try {
        response = (await onError(error)) ?? internalServerError()
      } catch (error) {
        console.error(`There was an error in the error handler: ${error}`)
        response = internalServerError()
      }
    }

    await sendResponse(res, response)
  }
}
```

### Error Handling — Double Try/Catch

```typescript
try {
  response = await handler(request, client)
} catch (error) {
  try {
    response = (await onError(error)) ?? internalServerError()
  } catch (error) {
    // Even the error handler failed
    console.error(`There was an error in the error handler: ${error}`)
    response = internalServerError()
  }
}
```

The inner try/catch ensures that even if the custom `onError` handler throws, a 500 response is still sent.

### internalServerError — Raw Bytes

```typescript
function internalServerError(): Response {
  return new Response(
    new Uint8Array([
      73, 110, 116, 101, 114, 110, 97, 108, 32, 83, 101, 114, 118, 101, 114, 32, 69, 114, 114, 111, 114,
    ]),  // "Internal Server Error"
    { status: 500, headers: { 'Content-Type': 'text/plain' } }
  )
}
```

Aha: The error message is stored as raw byte values, not a string literal. This avoids any potential string encoding issues and ensures the response is always valid ASCII.

## createRequest — Node.js → Web Request

```typescript
export function createRequest(req, res, options?): Request {
  let controller: AbortController | null = new AbortController()

  // Abort when client disconnects before response is sent
  res.once('close', () => controller?.abort())
  res.once('finish', () => (controller = null))

  let method = req.method ?? 'GET'
  let headers = createHeaders(req)
  let protocol = options?.protocol ?? ('encrypted' in req.socket && req.socket.encrypted ? 'https:' : 'http:')
  let host = options?.host ?? headers.get('Host') ?? 'localhost'
  let url = new URL(req.url!, `${protocol}//${host}`)

  let init: RequestInit = { method, headers, signal: controller.signal }

  if (method !== 'GET' && method !== 'HEAD') {
    init.body = new ReadableStream({
      start(controller) {
        req.on('data', (chunk) => {
          controller.enqueue(new Uint8Array(chunk.buffer, chunk.byteOffset, chunk.byteLength))
        })
        req.on('end', () => controller.close())
      },
    })
    ;(init as { duplex: 'half' }).duplex = 'half'
  }

  return new Request(url, init)
}
```

### Abort Signal — Client Disconnect Detection

```typescript
res.once('close', () => controller?.abort())    // Client disconnected
res.once('finish', () => (controller = null))     // Response fully sent
```

If the client disconnects before the handler finishes, the request's `AbortSignal` fires. The handler can use `request.signal` to cancel expensive operations.

After `finish` (response fully sent), `controller` is nulled to allow garbage collection.

### Body Stream — Event-Based

```typescript
init.body = new ReadableStream({
  start(controller) {
    req.on('data', (chunk) => {
      controller.enqueue(new Uint8Array(chunk.buffer, chunk.byteOffset, chunk.byteLength))
    })
    req.on('end', () => controller.close())
  },
})
```

Converts Node.js `data`/`end` events to a `ReadableStream`. The `Uint8Array` construction uses `chunk.buffer` with offset/length to avoid copying — it references the same underlying `ArrayBuffer`.

## createHeaders

```typescript
export function createHeaders(req): Headers {
  let headers = new Headers()
  let rawHeaders = req.rawHeaders  // [name, value, name, value, ...]

  for (let i = 0; i < rawHeaders.length; i += 2) {
    if (rawHeaders[i].startsWith(':')) continue  // Skip HTTP/2 pseudo-headers
    headers.append(rawHeaders[i], rawHeaders[i + 1])
  }

  return headers
}
```

Uses `req.rawHeaders` (flat array) instead of `req.headers` (object) to preserve duplicate headers and header order.

## sendResponse — Web Response → Node.js

```typescript
export async function sendResponse(res, response): Promise<void> {
  // Build headers object, handling duplicates (Set-Cookie)
  let headers: Record<string, string | string[]> = {}
  for (let [key, value] of response.headers) {
    if (key in headers) {
      if (Array.isArray(headers[key])) {
        headers[key].push(value)
      } else {
        headers[key] = [headers[key] as string, value]
      }
    } else {
      headers[key] = value
    }
  }

  // HTTP/1 vs HTTP/2: HTTP/2 doesn't support status messages
  if (res.req.httpVersionMajor === 1) {
    res.writeHead(response.status, response.statusText, headers)
  } else {
    res.writeHead(response.status, headers)
  }

  // Stream response body
  if (response.body != null && res.req.method !== 'HEAD') {
    for await (let chunk of readStream(response.body)) {
      if (res.write(chunk) === false) {
        // Backpressure: wait for drain
        await new Promise<void>((resolve) => res.once('drain', resolve))
      }
    }
  }

  res.end()
}
```

### Backpressure Handling

```typescript
if (res.write(chunk) === false) {
  await new Promise<void>((resolve) => res.once('drain', resolve))
}
```

When `res.write()` returns `false`, the internal buffer is full. Waiting for the `drain` event prevents memory buildup from writing faster than the client can read.

Aha: The header building iterates `response.headers` instead of using `Object.fromEntries(response.headers.entries())`. This is critical for `Set-Cookie` — multiple Set-Cookie headers must be sent as separate headers, not comma-joined. The iteration preserves duplicates.

## Cross-References

- [Fetch Router](09-node-fetch-server.md) — Router that uses this as server backend
