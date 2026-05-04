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

## 4. fetch — SSE-based HTTP client

The most complex action plugin. Creates HTTP method actions (`@get`, `@post`, `@put`, `@patch`, `@delete`) that communicate with servers via Server-Sent Events.

```html
<!-- Simple GET -->
<button data-on:click="@get('/api/users')">Load</button>

<!-- POST with JSON content type -->
<button data-on:click="@post('/api/save', { contentType: 'json' })">Save</button>

<!-- Form submission -->
<form data-on:submit="@post('/api/submit', { contentType: 'form' })">
  <input name="email" />
</form>

<!-- With headers and retry -->
<button data-on:click="@post('/api/data', {
  headers: { 'X-Custom': 'value' },
  retry: 'auto',
  retryInterval: 1000,
  retryMaxCount: 10
})">Send</button>
```

### FetchArgs

| Option | Type | Default | Purpose |
|--------|------|---------|---------|
| `selector` | `string` | — | CSS selector for form element |
| `headers` | `Record<string, string>` | `{}` | Custom headers |
| `contentType` | `'json' \| 'form'` | `'json'` | Request body encoding |
| `filterSignals` | `{ include, exclude }` | `{ include: /.*/, exclude: /(^|\\.)_/ }` | Which signals to send |
| `openWhenHidden` | `boolean` | `true` (GET), `true` (others) | Continue SSE when tab hidden |
| `payload` | `any` | `filtered(...)` | Custom payload instead of signals |
| `requestCancellation` | `'auto' \| 'cleanup' \| 'disabled' \| AbortController` | `'auto'` | How to handle abort |
| `retry` | `'auto' \| 'error' \| 'always' \| 'never'` | `'auto'` | Retry strategy |
| `retryInterval` | `number` | `1000` | Initial retry delay (ms) |
| `retryScaler` | `number` | `2` | Exponential backoff multiplier |
| `retryMaxWait` | `number` | `30000` | Max retry delay (ms) |
| `retryMaxCount` | `number` | `10` | Max number of retries |

### Content Types

**JSON mode**: Signals are serialized via `filtered()` and sent as JSON body (or URL query param for GET/DELETE).

```typescript
startPeeking()
const requestPayload = payload !== undefined ? payload : filtered({ include, exclude })
stopPeeking()
const body = JSON.stringify(requestPayload)
```

**Form mode**: Collects form data via `FormData`, validates the form, and sends as `application/x-www-form-urlencoded` or `multipart/form-data`.

### SSE Event Handling

The fetch plugin parses the SSE response and dispatches custom events:

| SSE Content-Type | Event dispatched | What it does |
|-----------------|-----------------|--------------|
| `text/event-stream` | `datastar-patch-elements` / `datastar-patch-signals` | Patches DOM or signals |
| `text/html` | Non-SSE dispatch | Dispatches `datastar-patch-elements` with HTML |
| `application/json` | Non-SSE dispatch | Dispatches `datastar-patch-signals` with JSON |
| `text/javascript` | Script execution | Creates and appends a `<script>` element |

### Retry Logic

```typescript
// Exponential backoff
retryInterval = Math.min(retryInterval * retryScaler, retryMaxWait)
if (++retries >= retryMaxCount) {
  dispatchFetch('retries-failed', el, {})
  reject('Max retries reached.')
}
```

The retry is reset on successful connection:

```typescript
// On successful connection, reset retry counter
retries = 0
retryInterval = baseRetryInterval
```

### Request Cancellation

| Mode | Behavior |
|------|----------|
| `auto` | Abort previous request on same element when new one starts |
| `cleanup` | Abort on element cleanup + after element removal (waits one tick) |
| `disabled` | No automatic cancellation |
| `AbortController` | Use a custom controller |

### Lifecycle Events

The fetch plugin dispatches `datastar-fetch` events on the document:

```typescript
dispatchFetch('started', el, {})      // Request begins
dispatchFetch('finished', el, {})     // Request completes (success or failure)
dispatchFetch('error', el, {...})     // HTTP error (4xx/5xx)
dispatchFetch('retrying', el, {...})  // About to retry
dispatchFetch('retries-failed', el, {}) // Max retries exceeded
```

These are consumed by the `indicator` attribute plugin.

**Aha:** The fetch plugin implements a complete SSE client from scratch — `getBytes`, `getLines`, `getMessages` — rather than using the browser's `EventSource` API. This is intentional: `EventSource` only supports GET requests. By building on `fetch` + `ReadableStream`, the plugin can send POST/PUT/PATCH/DELETE with request bodies while still receiving SSE responses.

See [SSE Streaming](08-sse-streaming.md) for the SSE parsing details.
See [Watchers](09-watchers.md) for how fetch results are handled.
