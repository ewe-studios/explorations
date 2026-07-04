# HTTP Headers: SuperHeaders and 14 Header Parsers

`@remix-run/headers` (v0.18.0) is a comprehensive HTTP headers toolkit: parsing, serialization, and type-safe manipulation of all standard HTTP headers.

Source: `packages/headers/src/` — 18 source files.

## SuperHeaders Class

Source: `lib/super-headers.ts`

```typescript
export class SuperHeaders extends Headers {
  #map: Map<string, string | HeaderValue>
  #setCookies: (string | SetCookie)[] = []

  constructor(init?: string | SuperHeadersInit | Headers) {
    super()
    this.#map = new Map()

    if (init) {
      if (typeof init === 'string') {
        // Raw header string: "Content-Type: text/html\r\nAccept: */*"
        let lines = init.split(CRLF)
        for (let line of lines) {
          let match = line.match(/^([^:]+):(.*)/)
          if (match) this.append(match[1].trim(), match[2].trim())
        }
      } else if (isIterable(init)) {
        // Iterable: [["Content-Type", "text/html"], ["Accept", "*/*"]]
        for (let [name, value] of init) this.append(name, value)
      } else if (typeof init === 'object') {
        // Property-based: { contentType: "text/html", accept: "*/*" }
        for (let name of Object.getOwnPropertyNames(init)) {
          let value = init[name]
          let descriptor = Object.getOwnPropertyDescriptor(SuperHeaders.prototype, name)
          if (descriptor?.set) {
            descriptor.set.call(this, value)  // Use typed setter
          } else {
            this.set(name, value.toString())  // Generic set
          }
        }
      }
    }
  }
```

### Three Init Modes

| Init Type | Example | Behavior |
|-----------|---------|----------|
| String | `"Content-Type: text/html\r\nAccept: */*"` | Parses raw header lines |
| Iterable | `[["Content-Type", "text/html"]]` | Appends each pair |
| Object | `{ accept: "text/html" }` | Uses typed setters if available |

Aha: The property-based init checks `Object.getOwnPropertyDescriptor(SuperHeaders.prototype, name)` to see if a typed setter exists. If `accept` is passed, it calls `this.accept = "text/html"` which creates an `Accept` object. This means `{ accept: "text/html, application/json" }` produces a parsed `Accept` with two media types.

### append()

```typescript
append(name: string, value: string): void {
  let key = name.toLowerCase()
  if (key === SetCookieKey) {
    this.#setCookies.push(value)
  } else {
    let existingValue = this.#map.get(key)
    this.#map.set(key, existingValue ? `${existingValue}, ${value}` : value)
  }
}
```

Set-Cookie is special: multiple Set-Cookie headers are stored in a separate array (RFC 6265 requires them to be separate headers, not comma-joined).

### get()

```typescript
get(name: string): string | null {
  let key = name.toLowerCase()
  if (key === SetCookieKey) {
    return this.getSetCookie().join(', ')
  } else {
    let value = this.#map.get(key)
    if (typeof value === 'string') return value
    if (value != null) {
      let str = value.toString()
      return str === '' ? null : str
    }
    return null
  }
}
```

For Set-Cookie, joins all values with `, `. For other headers, converts `HeaderValue` objects to strings.

## Typed Getters

### getAccept()

```typescript
getAccept(): Accept[] {
  let value = this.#map.get('accept')
  return value instanceof Accept ? [value] : []
}
```

### Accept Class

Source: `lib/accept.ts`

```typescript
export class Accept implements HeaderValue, Iterable<[string, number]> {
  #map: Map<string, number>

  constructor(init?: string | AcceptInit) {
    this.#map = new Map()
    if (init) {
      if (typeof init === 'string') {
        for (let piece of init.split(/\s*,\s*/)) {
          let params = parseParams(piece)
          let mediaType = params[0][0]
          let weight = 1
          for (let i = 1; i < params.length; i++) {
            let [key, value] = params[i]
            if (key === 'q') { weight = Number(value); break }
          }
          this.#map.set(mediaType.toLowerCase(), weight)
        }
      } else if (isIterable(init)) {
        for (let mediaType of init) {
          if (Array.isArray(mediaType)) {
            this.#map.set(mediaType[0].toLowerCase(), mediaType[1])
          } else {
            this.#map.set(mediaType.toLowerCase(), 1)
          }
        }
      } else {
        for (let mediaType of Object.getOwnPropertyNames(init)) {
          this.#map.set(mediaType.toLowerCase(), init[mediaType])
        }
      }
      this.#sort()
    }
  }

  #sort() {
    this.#map = new Map([...this.#map].sort((a, b) => b[1] - a[1]))
  }
```

Aha: Accept entries are auto-sorted by weight (q value), highest first. This means `accept.mediaTypes` returns types in preference order without additional sorting.

### ContentNegotiation

```typescript
negotiate(available: string[]): string | null {
  for (let mediaType of this.mediaTypes) {
    if (mediaType === '*/*') return available[0]
    if (mediaType.endsWith('/*')) {
      let prefix = mediaType.slice(0, mediaType.length - 1)
      let match = available.find(m => m.startsWith(prefix))
      if (match) return match
    } else {
      if (available.includes(mediaType)) return mediaType
    }
  }
  return null
}
```

Content negotiation: finds the best match from available types. Supports `*/*` and `type/*` wildcards.

## Cache-Control

Source: `lib/cache-control.ts`

```typescript
export class CacheControl implements HeaderValue {
  noCache?: true
  noStore?: true
  maxAge?: number
  sMaxage?: number
  staleWhileRevalidate?: number
  // ... 12 directives total
}
```

### toString() — Serialization

```typescript
toString(): string {
  let parts: string[] = []
  if (this.noCache) parts.push('no-cache')
  if (this.noStore) parts.push('no-store')
  if (this.maxAge !== undefined) parts.push(`max-age=${this.maxAge}`)
  if (this.sMaxage !== undefined) parts.push(`s-maxage=${this.sMaxage}`)
  if (this.staleWhileRevalidate !== undefined) parts.push(`stale-while-revalidate=${this.staleWhileRevalidate}`)
  // ... all directives
  return parts.join(', ')
}
```

Boolean directives (noCache, noStore, etc.) produce just the name. Numeric directives produce `name=value`.

## Set-Cookie

Source: `lib/set-cookie.ts`

```typescript
export class SetCookie implements HeaderValue, CookieProperties {
  name?: string
  value?: string
  domain?: string
  path?: string
  expires?: Date
  maxAge?: number
  secure?: boolean
  httpOnly?: boolean
  sameSite?: 'Strict' | 'Lax' | 'None'
  partitioned?: boolean

  toString(): string {
    let parts = []
    if (this.name !== undefined) parts.push(`${this.name}=${this.value ?? ''}`)
    if (this.domain) parts.push(`Domain=${this.domain}`)
    if (this.path) parts.push(`Path=${this.path}`)
    if (this.expires) parts.push(`Expires=${this.expires.toUTCString()}`)
    if (this.maxAge !== undefined) parts.push(`Max-Age=${this.maxAge}`)
    if (this.secure) parts.push('Secure')
    if (this.httpOnly) parts.push('HttpOnly')
    if (this.sameSite) parts.push(`SameSite=${this.sameSite}`)
    if (this.partitioned) parts.push('Partitioned')
    return parts.join('; ')
  }
}
```

## ETag — if-match / if-none-match

Source: `lib/if-match.ts`, `lib/if-none-match.ts`

```typescript
export class IfMatch implements HeaderValue, Iterable<EntityTag> {
  #tags: EntityTag[]

  constructor(init?: string | IfMatchInit) {
    this.#tags = []
    if (init) {
      if (typeof init === 'string') {
        for (let piece of init.split(/\s*,\s*/)) {
          this.#tags.push(parseEntityTag(piece))
        }
      } else if (isIterable(init)) {
        for (let tag of init) this.#tags.push(toEntityTag(tag))
      }
    }
  }
}

interface EntityTag {
  tag: string
  weak: boolean  // W/"xyz" is weak, "xyz" is strong
}
```

### parseEntityTag()

```typescript
function parseEntityTag(value: string): EntityTag {
  let weak = false
  if (value.startsWith('W/')) {
    weak = true
    value = value.slice(2)
  }
  if (value.startsWith('"') && value.endsWith('"')) {
    value = value.slice(1, -1)
  }
  return { tag: value, weak }
}
```

Strips `W/` prefix and quotes.

## Content-Range

Source: `lib/content-range.ts`

```typescript
export class ContentRange implements HeaderValue {
  unit: string      // 'bytes'
  start: number
  end: number
  size: number | '*'

  toString(): string {
    return `${this.unit} ${this.start}-${this.end}/${this.size}`
  }
}
```

Parses: `Content-Range: bytes 0-99/200` → `{ unit: 'bytes', start: 0, end: 99, size: 200 }`.

## Range

Source: `lib/range.ts`

```typescript
export class Range implements HeaderValue, Iterable<RangeUnit> {
  #units: RangeUnit[]

  // Parses: Range: bytes=0-99, 200-299
}

interface RangeUnit {
  start: number
  end?: number  // undefined means "to end"
}
```

## All Typed Getters

| Getter | Returns | Parses |
|--------|---------|--------|
| `getAccept()` | `Accept[]` | `Accept: text/html, application/json;q=0.9` |
| `getAcceptEncoding()` | `AcceptEncoding[]` | `Accept-Encoding: gzip, br` |
| `getAcceptLanguage()` | `AcceptLanguage[]` | `Accept-Language: en-US, en;q=0.9` |
| `getCacheControl()` | `CacheControl` | `Cache-Control: no-cache, max-age=0` |
| `getContentDisposition()` | `ContentDisposition` | `Content-Disposition: attachment; filename="doc.pdf"` |
| `getContentRange()` | `ContentRange` | `Content-Range: bytes 0-99/200` |
| `getContentType()` | `ContentType` | `Content-Type: text/html; charset=utf-8` |
| `getCookie()` | `Cookie[]` | `Cookie: session=abc; theme=dark` |
| `getIfMatch()` | `IfMatch` | `If-Match: "abc", W/"weak"` |
| `getIfNoneMatch()` | `IfNoneMatch` | `If-None-Match: "abc"` |
| `getIfRange()` | `IfRange` | `If-Range: "abc"` or `If-Range: Wed, 21 Oct 2015 07:28:00 GMT` |
| `getRange()` | `Range` | `Range: bytes=0-99, 200-299` |
| `getSetCookie()` | `SetCookie[]` | `Set-Cookie: session=abc; Path=/; HttpOnly` |
| `getVary()` | `Vary` | `Vary: Accept-Encoding, Accept-Language` |

## Package Structure

```
headers/src/
├── index.ts                   # Public API, SuperHeaders
└── lib/
    ├── super-headers.ts       # SuperHeaders class (all getters/setters)
    ├── accept.ts              # Accept header (content negotiation)
    ├── accept-encoding.ts     # Accept-Encoding (gzip, br, deflate)
    ├── accept-language.ts     # Accept-Language (locale preference)
    ├── cache-control.ts       # Cache-Control (12 directives)
    ├── content-disposition.ts # Content-Disposition (inline/attachment)
    ├── content-range.ts       # Content-Range (partial responses)
    ├── content-type.ts        # Content-Type (media type + params)
    ├── cookie.ts              # Cookie request header parsing
    ├── set-cookie.ts          # Set-Cookie response header (10 attributes)
    ├── if-match.ts            # If-Match (strong/weak ETags)
    ├── if-none-match.ts       # If-None-Match (conditional GET)
    ├── if-range.ts            # If-Range (ETag or date)
    ├── range.ts               # Range (byte ranges)
    ├── vary.ts                # Vary (cache variation)
    ├── header-names.ts        # Canonical header name formatting
    ├── header-value.ts        # HeaderValue interface
    ├── param-values.ts        # Parameter parsing (key=value pairs)
    └── utils.ts               # Shared utilities (quoteEtag, isIterable)
```

## Cross-References

- [Overview](00-overview.md) — What Remix is
- [Cookie](05-cookie.md) — Cookie toolkit (separate from Cookie header parsing)
- [Response](07-response.md) — Response helpers (uses headers)

[Next → Response](07-response.md)
