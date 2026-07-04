# Fetch Proxy

`@remix-run/fetch-proxy` creates a `fetch` function that forwards requests to another server, rewriting cookies and adding forwarded headers.

Source: `packages/fetch-proxy/src/lib/fetch-proxy.ts` — 104 lines.

## createFetchProxy

```typescript
export function createFetchProxy(target: string | URL, options?: FetchProxyOptions): FetchProxy {
  let localFetch = options?.fetch ?? globalThis.fetch
  let rewriteCookieDomain = options?.rewriteCookieDomain ?? true
  let rewriteCookiePath = options?.rewriteCookiePath ?? true
  let xForwardedHeaders = options?.xForwardedHeaders ?? false

  let targetUrl = new URL(target)
  if (targetUrl.pathname.endsWith('/')) {
    targetUrl.pathname = targetUrl.pathname.replace(/\/+$/, '')
  }

  return async (input, init) => {
    // ...
  }
}
```

### URL Rewriting

```typescript
let proxyUrl = new URL(url.search, targetUrl)
if (url.pathname !== '/') {
  proxyUrl.pathname =
    proxyUrl.pathname === '/' ? url.pathname : proxyUrl.pathname + url.pathname
}
```

Examples:
| Incoming | Target | Proxied To |
|----------|--------|-----------|
| `http://shopify.com` | `https://remix.run:3000/dest` | `https://remix.run:3000/dest` |
| `http://shopify.com/?q=remix` | `https://remix.run:3000/dest` | `https://remix.run:3000/dest?q=remix` |
| `http://shopify.com/search?q=remix` | `https://remix.run:3000/` | `https://remix.run:3000/search?q=remix` |
| `http://shopify.com/search?q=remix` | `https://remix.run:3000/dest` | `https://remix.run:3000/dest/search?q=remix` |

Aha: The search string is always preserved. The target's pathname and the incoming pathname are concatenated (avoiding double slashes). This means you can proxy `/api/*` to `https://api.example.com/api/*` transparently.

### Request Forwarding

```typescript
let proxyInit: RequestInit = {
  method: request.method,
  headers: proxyHeaders,
  cache: request.cache,
  credentials: request.credentials,
  integrity: request.integrity,
  keepalive: request.keepalive,
  mode: request.mode,
  redirect: request.redirect,
  referrer: request.referrer,
  referrerPolicy: request.referrerPolicy,
  signal: request.signal,
  ...init,
}
if (request.method !== 'GET' && request.method !== 'HEAD') {
  proxyInit.body = request.body
  ;(proxyInit as { duplex: 'half' }).duplex = 'half'
}
```

All `Request` properties are forwarded. The `duplex: 'half'` is required when the body is a `ReadableStream` (per the Fetch spec).

### Cookie Domain Rewriting

```typescript
if (responseHeaders.has('Set-Cookie')) {
  let setCookie = responseHeaders.getSetCookie()
  responseHeaders.delete('Set-Cookie')

  for (let cookie of setCookie) {
    let header = new SetCookie(cookie)

    if (rewriteCookieDomain && header.domain) {
      header.domain = url.host  // Replace target domain with incoming domain
    }

    if (rewriteCookiePath && header.path) {
      if (header.path.startsWith(targetUrl.pathname + '/')) {
        header.path = header.path.slice(targetUrl.pathname.length)
      } else if (header.path === targetUrl.pathname) {
        header.path = '/'
      }
    }

    responseHeaders.append('Set-Cookie', header.toString())
  }
}
```

Uses `@remix-run/headers` `SetCookie` class for proper parsing and serialization.

### X-Forwarded Headers

```typescript
if (xForwardedHeaders) {
  proxyHeaders.append('X-Forwarded-Proto', url.protocol.replace(/:$/, ''))
  proxyHeaders.append('X-Forwarded-Host', url.host)
}
```

Adds `X-Forwarded-Proto` (http/https) and `X-Forwarded-Host` (incoming host) to the proxied request.

## Cross-References

- [Fetch Router](09-node-fetch-server.md) — Router that can use fetch proxy
- [Headers](06-headers.md) — SetCookie class used for cookie rewriting
