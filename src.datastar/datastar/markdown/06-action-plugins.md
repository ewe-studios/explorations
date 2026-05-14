# Datastar -- Action Plugins

Four action plugins are invoked via `@actionName(...)` syntax in expressions. Unlike attribute plugins which are bound to elements, actions are invoked imperatively.

Source: `library/src/plugins/actions/*.ts`

## 1. peek — Read without subscribing

```html
<!-- data-on:click="$total = peek(() => $count * 2)" -->
```

```typescript
// plugins/actions/peek.ts
action({
  name: 'peek',
  apply(_, fn: () => any) {
    startPeeking()
    try { return fn() }
    finally { stopPeeking() }
  },
})
```

Wraps the function call in `startPeeking()` / `stopPeeking()` so that any signal reads inside `fn` do NOT create dependency links.

## 2. setAll — Bulk set signals

Sets all matching signals to the same value:

```html
<!-- data-on:click="@setAll(false)" → sets ALL signals to false -->
<!-- data-on:click="@setAll(false, { include: /^form/ })" → only form signals -->
```

```typescript
// plugins/actions/setAll.ts
action({
  name: 'setAll',
  apply(_, value: any, filter: SignalFilterOptions) {
    startPeeking()
    const masked = filtered(filter)
    updateLeaves(masked, () => value)
    mergePatch(masked)
    stopPeeking()
  },
})
```

Uses `updateLeaves` to walk the masked object and set every leaf value to `value`, then `mergePatch` to apply the changes to the signal store.

## 3. toggleAll — Bulk toggle booleans

Like `setAll` but toggles each matching signal's boolean value:

```html
<!-- data-on:click="@toggleAll({ include: /^checkbox/ })" -->
```

```typescript
// plugins/actions/toggleAll.ts
action({
  name: 'toggleAll',
  apply(_, filter: SignalFilterOptions) {
    startPeeking()
    const masked = filtered(filter)
    updateLeaves(masked, (oldValue: any) => !oldValue)
    mergePatch(masked)
    stopPeeking()
  },
})
```

## 4. fetch — SSE-based HTTP client (Full Source Walkthrough)

Source: `plugins/actions/fetch.ts` — 693 lines

### createHttpMethod Factory (lines 20-227)

All HTTP methods are created from a single factory function:

```typescript
const createHttpMethod = (
  name: string,
  method: string,
  openWhenHiddenDefault: boolean = true,
): void =>
  action({
    name,
    apply: async ({ el, evt, error, cleanups }, url, { ...options }: FetchArgs = {}) => {
      // Full implementation
    },
  })

createHttpMethod('get', 'GET', false)    // GET defaults to close on hidden
createHttpMethod('patch', 'PATCH')
createHttpMethod('post', 'POST')
createHttpMethod('put', 'PUT')
createHttpMethod('delete', 'DELETE')     // DELETE has no request body
```

**Aha:** GET uses `openWhenHiddenDefault = false` because GET requests are typically for data loading, which shouldn't continue when the user navigates away. POST/PUT/PATCH default to `true` because they're often long-running server processes that should continue receiving events.

### Request Cancellation (lines 45-60)

```typescript
const controller = requestCancellation instanceof AbortController
  ? requestCancellation : new AbortController()
if (requestCancellation === 'auto' || requestCancellation === 'cleanup') {
  abortControllers.get(el)?.abort()  // Abort previous request on same element
  abortControllers.set(el, controller)
}
if (requestCancellation === 'cleanup') {
  cleanups.get(`@${name}`)?.()
  cleanups.set(`@${name}`, async () => {
    controller.abort()
    await Promise.resolve()  // Wait one tick for FINISHED to fire
  })
}
```

The `abortControllers` WeakMap tracks the current request per element. `'auto'` mode aborts the previous request immediately. `'cleanup'` mode defers abort until the element is removed from the DOM, with a `Promise.resolve()` tick delay to let the `FINISHED` event fire.

### Default Headers (lines 69-76)

```typescript
const initialHeaders: Record<string, any> = {
  Accept: 'text/event-stream, text/html, application/json',
  'Datastar-Request': true,
}
if (contentType === 'json' && methodSupportsRequestBody(method)) {
  initialHeaders['Content-Type'] = 'application/json'
}
const headers = Object.assign({}, initialHeaders, userHeaders)
```

The `Accept` header tells the server that Datastar can handle SSE, HTML, or JSON responses. The `Datastar-Request: true` header lets the server know this is a Datastar client (useful for server-side detection).

### Content Type Routing (lines 127-206)

**JSON mode** (lines 130-140):
```typescript
if (contentType === 'json') {
  startPeeking()
  const requestPayload = payload !== undefined ? payload : filtered({ include, exclude })
  stopPeeking()
  const body = JSON.stringify(requestPayload)
  if (methodSupportsRequestBody(method)) {
    req.body = body
  } else {
    queryParams.set('datastar', body)  // GET/DELETE: put in query param
  }
}
```

**Aha:** For GET and DELETE requests, the JSON payload is URL-encoded as `?datastar={json}` instead of a request body. This is because GET/DELETE semantically shouldn't have bodies per HTTP specs.

**Form mode** (lines 141-199):
```typescript
if (contentType === 'form') {
  const formEl = (selector ? document.querySelector(selector) : el.closest('form')) as HTMLFormElement
  if (!formEl) throw error('FetchFormNotFound')

  // Validate
  if (!formEl.noValidate && !formEl.checkValidity()) {
    formEl.reportValidity()
    return  // Abort if validation fails
  }

  // Collect FormData
  const formData = new FormData(formEl)
  // ... append submitter value if applicable ...

  const multipart = formEl.getAttribute('enctype') === 'multipart/form-data'
  if (!multipart) {
    headers['Content-Type'] = 'application/x-www-form-urlencoded'
  }
  // For GET/DELETE: append to query params
  // For POST/PUT/PATCH: set as body
}
```

### Response Content-Type Routing (lines 585-626)

After getting a 200 response, the content type determines handling:

```typescript
const ct = response.headers.get('Content-Type')
if (ct?.includes('text/html')) {
  return await dispatchNonSSE('datastar-patch-elements', response, 'elements', ...)
}
if (ct?.includes('application/json')) {
  return await dispatchNonSSE('datastar-patch-signals', response, 'signals', ...)
}
if (ct?.includes('text/javascript')) {
  const script = document.createElement('script')
  // Apply datastar-script-attributes header if present
  script.textContent = await response.text()
  document.head.appendChild(script)
  return
}
// Otherwise: parse as SSE stream
await getBytes(response.body!, getLines(getMessages(...)))
```

**What:** Non-SSE responses are dispatched to the appropriate watcher. HTML → `datastar-patch-elements`, JSON → `datastar-patch-signals`, JavaScript → direct script injection. Only `text/event-stream` content triggers the full SSE parser.

### fetchEventSource — Custom SSE Client (lines 455-692)

The SSE client is a reimplementation of Azure's `fetch-event-source`, adapted for Datastar's needs.

#### getBytes (lines 297-307)

```typescript
const getBytes = async (stream: ReadableStream<Uint8Array>, onChunk) => {
  const reader = stream.getReader()
  let result = await reader.read()
  while (!result.done) {
    onChunk(result.value)
    result = await reader.read()
  }
}
```

Reads the `ReadableStream` chunk by chunk, calling `onChunk` for each `Uint8Array`.

#### getLines (lines 309-371)

A stateful line parser that handles partial chunks:

```typescript
const getLines = (onLine) => {
  let buffer: Uint8Array | undefined
  let position = 0
  let fieldLength = -1
  let discardTrailingNewline = false

  return (arr: Uint8Array) => {
    // Append new bytes to buffer
    buffer = buffer ? concat(buffer, arr) : arr

    while (position < buffer.length) {
      // Handle \r\n sequences
      if (discardTrailingNewline) {
        if (buffer[position] === 10) { position++; discardTrailingNewline = false }
      }

      // Scan for line end
      for (; position < buffer.length; ++position) {
        if (buffer[position] === 58 /* : */) {
          if (fieldLength === -1) fieldLength = position - lineStart  // First colon
        } else if (buffer[position] === 13 /* \r */) {
          discardTrailingNewline = true
          // fallthrough to \n
        } else if (buffer[position] === 10 /* \n */) {
          // Line complete
          onLine(buffer.subarray(lineStart, position), fieldLength)
          lineStart = position; fieldLength = -1
          break
        }
      }
    }

    // Compact buffer: remove processed bytes
    if (lineStart === buffer.length) buffer = undefined
    else if (lineStart) { buffer = buffer.subarray(lineStart); position -= lineStart }
  }
}
```

**Aha:** The parser handles partial chunks — a single SSE line might span multiple `Uint8Array` chunks. The `buffer` accumulates bytes across calls, and `subarray` compacts processed bytes without copying. Byte values are used directly (58 = `:`, 13 = `\r`, 10 = `\n`) for performance over string comparison.

#### getMessages (lines 373-416)

Parses SSE field lines into `EventSourceMessage` objects:

```typescript
const getMessages = (onId, onRetry, onMessage) => {
  let message = newMessage()
  const decoder = new TextDecoder()

  return (line, fieldLength) => {
    if (!line.length) {
      // Empty line = end of message
      onMessage?.(message)
      message = newMessage()
    } else if (fieldLength > 0) {
      const field = decoder.decode(line.subarray(0, fieldLength))
      const valueOffset = fieldLength + (line[fieldLength + 1] === 32 ? 2 : 1)
      const value = decoder.decode(line.subarray(valueOffset))

      switch (field) {
        case 'data': message.data = message.data ? `${message.data}\n${value}` : value; break
        case 'event': message.event = value; break
        case 'id': onId(message.id = value); break
        case 'retry': onRetry(message.retry = +value); break
      }
    }
  }
}
```

Per the SSE spec, field format is `field: value` or `field:value` — the parser checks if the character after the colon+1 is a space (32) to skip the optional leading space.

#### Retry Logic (lines 659-686)

```typescript
catch (err) {
  if (!curRequestSignal.aborted) {
    const interval = onerror?.(err) || retryInterval
    clearTimeout(retryTimer)
    retryTimer = setTimeout(create, interval)
    retryInterval = Math.min(retryInterval * retryScaler, retryMaxWait)  // Exponential backoff
    if (++retries >= retryMaxCount) {
      dispatchFetch(RETRIES_FAILED, el, {})
      reject('Max retries reached.')
    }
  }
}
```

Exponential backoff: each retry multiplies the delay by `retryScaler` (default 2), capped at `retryMaxWait` (default 30s). After `retryMaxCount` (default 10) failures, it rejects.

### Visibility Change Handling (lines 489-503)

```typescript
const onVisibilityChange = () => {
  curRequestController.abort()  // Close current request
  if (!document.hidden) {
    const currentFetchInit = buildFetchEventSourceInit()
    input = currentFetchInit.input
    rest.body = currentFetchInit.body
    create()  // Re-establish connection when tab becomes visible
  }
}
if (!openWhenHidden) {
  document.addEventListener('visibilitychange', onVisibilityChange)
}
```

When the tab goes hidden and `openWhenHidden = false`, the connection is closed. When it becomes visible again, a new connection is established with fresh request parameters.

See [SSE Streaming](08-sse-streaming.md) for the SSE parsing details.
See [Watchers](09-watchers.md) for how fetch results are handled.
